cargo install basic-http-server

echo "open http://localhost:8080"

cd docs
basic-http-server --addr 127.0.0.1:8080 .
