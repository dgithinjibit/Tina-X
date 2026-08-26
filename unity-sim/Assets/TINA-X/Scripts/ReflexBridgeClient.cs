// Thin TCP client that talks the line-delimited JSON contract to the Rust reflex loop.
//
// Responsibilities (kept small on purpose):
//   * connect to 127.0.0.1:8765 (with reconnect),
//   * send one GyroReading line, read one ActuatorCommand line, per call,
//   * never block the physics thread indefinitely (bounded read timeout).
//
// It holds NO control logic — that all lives in Rust (rust-core). See BRIDGE_CONTRACT.md.
//
// NOTE: scaffold — not compiled in this repo's dev container. Written for a junior dev to read.

using System;
using System.IO;
using System.Net.Sockets;
using UnityEngine;

namespace TINA-X
{
    public sealed class ReflexBridgeClient : IDisposable
    {
        private readonly string _host;
        private readonly int _port;
        private TcpClient _client;
        private NetworkStream _stream;
        private StreamReader _reader;
        private StreamWriter _writer;

        public bool Connected => _client != null && _client.Connected;

        public ReflexBridgeClient(string host = "127.0.0.1", int port = 8765, int readTimeoutMs = 5)
        {
            _host = host;
            _port = port;
            _readTimeoutMs = readTimeoutMs;
        }

        private readonly int _readTimeoutMs;

        /// <summary>Open (or re-open) the connection. Safe to call repeatedly; no-op if connected.</summary>
        public void EnsureConnected()
        {
            if (Connected) return;
            try
            {
                _client = new TcpClient();
                _client.Connect(_host, _port);
                _client.NoDelay = true; // we want each tiny frame sent immediately
                _stream = _client.GetStream();
                _stream.ReadTimeout = _readTimeoutMs; // bounded: never hang the physics thread
                _reader = new StreamReader(_stream);
                _writer = new StreamWriter(_stream) { AutoFlush = true };
            }
            catch (Exception e)
            {
                Debug.LogWarning($"[TINA-X] reflex bridge connect failed: {e.Message}");
                Dispose();
            }
        }

        /// <summary>
        /// Send one reading and get the command back. Returns false if the exchange failed
        /// (caller should then apply zero torque for this tick and try again next tick).
        /// </summary>
        public bool Exchange(in GyroReading reading, out ActuatorCommand command)
        {
            command = default;
            EnsureConnected();
            if (!Connected) return false;

            try
            {
                // Unity -> Rust: one JSON object, newline-terminated (the contract).
                _writer.Write(JsonUtility.ToJson(reading));
                _writer.Write('\n');

                // Rust -> Unity: read exactly one line.
                string line = _reader.ReadLine();
                if (string.IsNullOrEmpty(line)) return false;

                command = JsonUtility.FromJson<ActuatorCommand>(line);

                // Cheap safety check: the reply must be for the reading we just sent.
                if (command.step != reading.step)
                {
                    Debug.LogWarning($"[TINA-X] step mismatch: sent {reading.step}, got {command.step}");
                    return false;
                }
                return true;
            }
            catch (Exception e)
            {
                // Timeout / disconnect: drop this frame, reconnect next tick (crash-tolerant).
                Debug.LogWarning($"[TINA-X] reflex exchange failed: {e.Message}");
                Dispose();
                return false;
            }
        }

        public void Dispose()
        {
            _reader?.Dispose();
            _writer?.Dispose();
            _stream?.Dispose();
            _client?.Close();
            _client = null;
            _stream = null;
            _reader = null;
            _writer = null;
        }
    }
}
