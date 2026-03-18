from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn

BODY = b'{"message":"hello","n":42}'

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(BODY)))
        self.end_headers()
        self.wfile.write(BODY)
    def log_message(self, *args):
        pass

class ThreadedServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True

ThreadedServer(('', 8765), Handler).serve_forever()
