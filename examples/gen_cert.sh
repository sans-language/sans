#!/bin/bash
# Generate a self-signed certificate for HTTPS testing
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes \
  -subj "/CN=localhost"
echo "Generated cert.pem and key.pem"
