#!/usr/bin/env python3
"""
Simple HTTP server for serving the WebRTC client files.
Run this script to serve the web client on http://localhost:8000
"""

import http.server
import socketserver
import os
import sys
from pathlib import Path

# Get the directory where this script is located
SCRIPT_DIR = Path(__file__).parent.absolute()


class CustomHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=SCRIPT_DIR, **kwargs)

    def end_headers(self):
        # Add CORS headers to allow WebRTC to work
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        super().end_headers()


def main():
    PORT = 8000

    print(f"Starting web server for WebRTC client...")
    print(f"Server directory: {SCRIPT_DIR}")
    print(f"Server URL: http://localhost:{PORT}")
    print(f"Press Ctrl+C to stop the server")
    print("-" * 50)

    try:
        with socketserver.TCPServer(("", PORT), CustomHTTPRequestHandler) as httpd:
            print(f"✅ Server started successfully on port {PORT}")
            print(f"🌐 Open your browser and go to: http://localhost:{PORT}")
            print(
                f"📝 Make sure your WebSocket signaling server is running on ws://127.0.0.1:8080"
            )
            print("-" * 50)
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n🛑 Server stopped by user")
    except OSError as e:
        if e.errno == 48:  # Address already in use
            print(f"❌ Error: Port {PORT} is already in use.")
            print(
                f"💡 Try using a different port or stop the service using port {PORT}"
            )
        else:
            print(f"❌ Error starting server: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
