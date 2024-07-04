#!/bin/sh

# Start Xvfb
Xvfb :1 -screen 0 1280x720x24 &

# Start Openbox
openbox &

# Start x11vnc
x11vnc -display :1 -nopw -forever -rfbport 5901 &

# Start websockify to connect noVNC to VNC server
websockify --web=/usr/share/novnc/ 8080 localhost:5900

# Keep container running
tail -f /dev/null
