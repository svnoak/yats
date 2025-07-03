from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs

class DetailedHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        # Parse the URL to get path and query parameters
        parsed_url = urlparse(self.path)
        path = parsed_url.path
        query_params = parse_qs(parsed_url.query)

        # Print the request details
        print(f"\n--- Incoming GET Request ---")
        print(f"Path: {path}")
        print(f"Query Parameters: {query_params}")
        print(f"Headers:")
        for header, value in self.headers.items():
            print(f"  {header}: {value}")
        print(f"--------------------------")

        # Send the 200 OK response
        self.send_response(200)
        self.send_header('Content-type', 'text/html')
        self.end_headers()
        self.wfile.write(b"200 OK - Details printed to console")

def run(server_class=HTTPServer, handler_class=DetailedHandler, port=8080):
    server_address = ('', port)
    httpd = server_class(server_address, handler_class)
    print(f"Starting httpd server on localhost:{port}")
    print("Press Ctrl+C to stop the server.")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    httpd.server_close()
    print("\nStopping httpd server.")

if __name__ == '__main__':
    run()