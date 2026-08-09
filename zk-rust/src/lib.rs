//! Rust-native ZK proof-of-concept for the "Verifiable Safe-to-Fly Decision" (see docs/zk/).
//!
//! # For a junior dev: what are we doing here?
//!
//! We want to prove, in zero knowledge, that an agent applied the safety rule correctly:
//!
//! ```text
//! decision d = 1 (fly) if wind w <= tolerance t, else 0 (refuse)
//! ```
//!
//! WITHOUT revealing the private wind reading `w`. We use **arkworks** with the **Groth16**
//! proving system over the **BN254** curve.
//!
//! The core idea of a zk-SNARK: we express the statement as an **arithmetic circuit** (a set of
//! constraints that are satisfied iff the statement is true), then Groth16 lets a prover produce
//! a short proof that they know private inputs satisfying the circuit, and a verifier check it
//! quickly using only the PUBLIC inputs.
//!
//! Public inputs here:  tolerance `t`, decision `d`.
//! Private witness:     wind `w`.
//!
//! # The comparison trick
//! Circuits can't "branch" like normal code. To enforce `w <= t` (or `w > t`), we work with the
//! difference and prove it fits in a fixed number of bits (i.e. is non-negative and bounded).
//! We keep values small (wind/tolerance in [0, 255]) so an 8-bit range check is enough, which
//! keeps this POC tiny and readable.

use ark_ff::PrimeField;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

/// Number of bits we allow wind/tolerance to occupy. 8 bits => values in [0, 255] m/s, plenty.
pub const VALUE_BITS: usize = 8;

/// The circuit describing one safe-to-fly decision.
///
/// Fields are `Option<..>` because the SAME circuit definition is used twice:
/// - at **setup** time we only need the shape (values can be `None`),
/// - at **proving** time we fill in the real values.
#[derive(Clone)]
pub struct SafeToFlyCircuit<F: PrimeField> {
    /// Private: the wind reading (witness). Hidden from the verifier.
    pub wind: Option<F>,
    /// Private: whether the position fix came from AUTHENTICATED Galileo signals — the OSNMA
    /// (Open Service Navigation Message Authentication) status, 1 = authenticated, 0 = not
    /// (G4D-RR bridge #1; see docs/research/g4drr-gnss-eo-bridge.md). A witness, not public: the
    /// verifier learns the drone *was* positioned by non-spoofed signals WITHOUT seeing any raw
    /// signal data. This is what makes "safe-to-fly" resistant to a spoofed-GNSS attack.
    pub authenticated: Option<F>,
    /// Public: the agent's published tolerance.
    pub tolerance: Option<F>,
    /// Public: the claimed decision (1 = fly, 0 = refuse).
    pub decision: Option<F>,
}

impl<F: PrimeField> ConstraintSynthesizer<F> for SafeToFlyCircuit<F> {
    /// Encode the statement as constraints. Called by arkworks during setup and proving.
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // 1. Allocate variables. `new_witness` = private, `new_input` = public.
        let wind = FpVar::new_witness(cs.clone(), || {
            self.wind.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let authenticated = FpVar::new_witness(cs.clone(), || {
            self.authenticated.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let tolerance = FpVar::new_input(cs.clone(), || {
            self.tolerance.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let decision = FpVar::new_input(cs.clone(), || {
            self.decision.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 2. Range-check wind and tolerance to VALUE_BITS bits. This both bounds them and gives
        //    us their little-endian bits, which we reuse for the comparison.
        let wind_bits = to_bits_le_checked(&wind, VALUE_BITS)?;
        let _tol_bits = to_bits_le_checked(&tolerance, VALUE_BITS)?;

        // 3. Compute `is_safe = (wind <= tolerance)` as a Boolean, purely with constraints.
        //    We use: wind <= tolerance  <=>  (tolerance - wind) is non-negative and fits in bits.
        //    Concretely: let diff = tolerance - wind + 2^VALUE_BITS. If wind <= tolerance then
        //    diff's bit VALUE_BITS is 1; if wind > tolerance it's 0. This is the classic
        //    "borrow bit" comparison.
        let two_pow = F::from(2u64).pow([VALUE_BITS as u64]);
        let offset = FpVar::constant(two_pow);
        let shifted = &tolerance - &wind + &offset; // in range [1, 2^(VALUE_BITS+1) - 1]
        let shifted_bits = to_bits_le_checked(&shifted, VALUE_BITS + 1)?;
        // The top bit tells us wind <= tolerance.
        let wind_ok = shifted_bits[VALUE_BITS].clone();

        // 3b. Force `authenticated` to be a genuine boolean (0 or 1): auth * (auth - 1) == 0. Without
        //     this a malicious prover could pass auth=2 etc. and short-circuit the AND below.
        let one = FpVar::constant(F::one());
        let auth_is_bool = &authenticated * (&authenticated - &one);
        auth_is_bool.enforce_equal(&FpVar::constant(F::zero()))?;

        // 4. Safe-to-fly requires BOTH conditions (bridge #1): within wind tolerance AND positioned
        //    by AUTHENTICATED (OSNMA) signals. AND of two 0/1 field values is their product.
        let wind_ok_f = FpVar::from(wind_ok);
        let is_safe = &wind_ok_f * &authenticated;

        // 5. Constrain the decision to equal our computed is_safe (a field element 0/1).
        decision.enforce_equal(&is_safe)?;

        // Keep `wind_bits` "used" so the range check isn't optimized as dead (it constrains wind).
        let _ = wind_bits;
        Ok(())
    }
}

/// Decompose `value` into exactly `n_bits` little-endian bits AND constrain that the bits
/// recompose to `value`. This simultaneously (a) proves `value < 2^n_bits` and (b) yields the
/// bits for arithmetic. Returns the Boolean bit variables.
fn to_bits_le_checked<F: PrimeField>(
    value: &FpVar<F>,
    n_bits: usize,
) -> Result<Vec<Boolean<F>>, SynthesisError> {
    // Ask arkworks for the value's bits (as witnesses), then enforce they rebuild `value`.
    let bits = value.to_bits_le()?;
    // `to_bits_le` returns full field-width bits; take the low n_bits and force the rest to 0.
    let (low, high) = bits.split_at(n_bits);
    for b in high {
        b.enforce_equal(&Boolean::FALSE)?;
    }
    Ok(low.to_vec())
}

// ---------------------------------------------------------------------------------------------
// A friendly high-level API so callers (and tests) don't touch arkworks internals directly.
// ---------------------------------------------------------------------------------------------

use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, ProvingKey};
use ark_snark::SNARK;

/// The decision rule in plain Rust — the ground truth the circuit must match. Safe-to-fly requires
/// BOTH the wind within tolerance AND an OSNMA-authenticated position fix (`authenticated` = true),
/// so a spoofed-GNSS agent (authenticated = false) can never be "safe" even in calm wind
/// (G4D-RR bridge #1).
pub fn safe_to_fly(wind: u64, tolerance: u64, authenticated: bool) -> u64 {
    if wind <= tolerance && authenticated {
        1
    } else {
        0
    }
}

/// A prover/verifier pair for the safe-to-fly statement.
pub struct SafeToFlyZk {
    pk: ProvingKey<Bn254>,
    pvk: PreparedVerifyingKey<Bn254>,
}

impl SafeToFlyZk {
    /// Run the trusted setup (once). For a POC we generate keys locally.
    pub fn setup() -> Result<Self, SynthesisError> {
        // Groth16 requires a CryptoRng. We use a fixed seed so the POC is reproducible; a real
        // deployment MUST use a secure OS RNG (rand::rngs::OsRng) instead.
        let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(0x4E5A_4900);
        // Setup only needs the circuit SHAPE, so all values are None.
        let circuit = SafeToFlyCircuit::<Fr> {
            wind: None,
            authenticated: None,
            tolerance: None,
            decision: None,
        };
        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)?;
        let pvk = Groth16::<Bn254>::process_vk(&vk)?;
        Ok(Self { pk, pvk })
    }

    /// Produce a proof that `decision` is the correct rule output for the (private) `wind` and
    /// (private) `authenticated` OSNMA status, given the (public) `tolerance`. The authentication
    /// status stays a WITNESS: the proof attests "positioned by authenticated signals" without
    /// revealing any signal data.
    pub fn prove(
        &self,
        wind: u64,
        authenticated: bool,
        tolerance: u64,
        decision: u64,
    ) -> Result<Proof<Bn254>, SynthesisError> {
        // Groth16 requires a CryptoRng. We use a fixed seed so the POC is reproducible; a real
        // deployment MUST use a secure OS RNG (rand::rngs::OsRng) instead.
        let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(0x4E5A_4900);
        let circuit = SafeToFlyCircuit::<Fr> {
            wind: Some(Fr::from(wind)),
            authenticated: Some(Fr::from(authenticated as u64)),
            tolerance: Some(Fr::from(tolerance)),
            decision: Some(Fr::from(decision)),
        };
        Groth16::<Bn254>::prove(&self.pk, circuit, &mut rng)
    }

    /// Verify a proof against the PUBLIC inputs only (tolerance, decision). The verifier never
    /// sees `wind`.
    pub fn verify(
        &self,
        tolerance: u64,
        decision: u64,
        proof: &Proof<Bn254>,
    ) -> Result<bool, SynthesisError> {
        // Public inputs must be provided in the SAME ORDER they were allocated in the circuit:
        // tolerance first, then decision.
        let public_inputs = vec![Fr::from(tolerance), Fr::from(decision)];
        Groth16::<Bn254>::verify_with_processed_vk(&self.pvk, &public_inputs, proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_rule_matches_expectations() {
        // Fly requires wind within tolerance AND an authenticated (OSNMA) position.
        assert_eq!(safe_to_fly(8, 12, true), 1); // safe wind + authenticated -> fly
        assert_eq!(safe_to_fly(12, 12, true), 1); // boundary is safe (<=)
        assert_eq!(safe_to_fly(20, 12, true), 0); // wind too high -> refuse
        assert_eq!(safe_to_fly(8, 12, false), 0); // calm but SPOOFED (unauthenticated) -> refuse
        assert_eq!(safe_to_fly(20, 12, false), 0); // both bad -> refuse
    }

    #[test]
    fn honest_safe_authenticated_decision_proves_and_verifies() {
        let zk = SafeToFlyZk::setup().unwrap();
        // wind=8 <= tol=12 AND authenticated -> decision should be 1
        let proof = zk.prove(8, true, 12, 1).unwrap();
        assert!(zk.verify(12, 1, &proof).unwrap());
    }

    #[test]
    fn honest_unsafe_decision_proves_and_verifies() {
        let zk = SafeToFlyZk::setup().unwrap();
        // wind=20 > tol=12 -> decision should be 0 (regardless of auth)
        let proof = zk.prove(20, true, 12, 0).unwrap();
        assert!(zk.verify(12, 0, &proof).unwrap());
    }

    #[test]
    fn spoofed_position_cannot_prove_safe_even_in_calm_wind() {
        // The bridge-#1 property: calm wind (8 <= 12) but UNAUTHENTICATED (spoofed) signals must
        // yield a refuse (0). An honest agent proves decision=0 fine...
        let zk = SafeToFlyZk::setup().unwrap();
        let proof = zk.prove(8, false, 12, 0).unwrap();
        assert!(zk.verify(12, 0, &proof).unwrap());
    }

    #[test]
    fn a_spoofed_agent_cannot_fabricate_a_fly_proof() {
        // ...and it CANNOT fabricate a "fly" (1) proof while unauthenticated: the constraints are
        // unsatisfiable (is_safe = wind_ok AND auth = 1*0 = 0 ≠ 1), so proving aborts. This is the
        // anti-spoofing guarantee OSNMA-as-a-witness buys us.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(|| {
            let zk = SafeToFlyZk::setup().unwrap();
            zk.prove(8, false, 12, 1) // calm wind, spoofed, claiming FLY -> impossible
        });
        std::panic::set_hook(prev_hook);
        assert!(result.is_err(), "an unauthenticated agent must not yield a valid fly proof");
    }

    #[test]
    fn lying_prover_cannot_make_a_proof() {
        // Claiming "fly" (1) when wind 20 > tol 12 is impossible: the circuit constraints are
        // unsatisfiable for that assignment, so no valid proof can exist. arkworks enforces this
        // by ABORTING inside `prove` (a debug assertion that the constraint system is satisfied),
        // so we assert that proving PANICS rather than silently producing a bad proof.
        //
        // Either way, the security property is the same: a dishonest agent cannot fabricate a
        // proof for a decision the rule doesn't support.
        // Suppress the (expected) panic backtrace so test output stays clean.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(|| {
            let zk = SafeToFlyZk::setup().unwrap();
            zk.prove(20, true, 12, 1)
        });
        std::panic::set_hook(prev_hook);
        assert!(result.is_err(), "a lying assignment must not yield a valid proof");
    }

    #[test]
    fn proof_for_one_decision_does_not_verify_as_another() {
        // An honest proof for decision=1 must not verify against a claimed decision=0.
        let zk = SafeToFlyZk::setup().unwrap();
        let proof = zk.prove(8, true, 12, 1).unwrap();
        assert!(!zk.verify(12, 0, &proof).unwrap());
    }
}
