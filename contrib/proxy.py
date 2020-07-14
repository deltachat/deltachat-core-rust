#!/usr/bin/env python3
# Examples:
#
# Original server that doesn't use SSL:
# ./proxy.py 8080 imap.nauta.cu 143
# ./proxy.py 8081 smtp.nauta.cu 25
#
# Original server that uses SSL:
# ./proxy.py 8080 testrun.org 993 --ssl
# ./proxy.py 8081 testrun.org 465 --ssl

from datetime import datetime
import argparse
import selectors
import ssl
import socket
import socketserver


class Proxy(socketserver.ThreadingTCPServer):
    allow_reuse_address = True

    def __init__(self, proxy_host, proxy_port, real_host, real_port, use_ssl):
        self.real_host = real_host
        self.real_port = real_port
        self.use_ssl = use_ssl
        super().__init__((proxy_host, proxy_port), RequestHandler)


class RequestHandler(socketserver.BaseRequestHandler):

    def handle(self):
        print('{} - {} CONNECTED.'.format(datetime.now(), self.client_address))

        total = 0
        real_server = (self.server.real_host, self.server.real_port)
        with socket.create_connection(real_server) as sock:
            if self.server.use_ssl:
                context = ssl.create_default_context()
                sock = context.wrap_socket(
                    sock, server_hostname=real_server[0])

            forward = {self.request: sock, sock: self.request}

            sel = selectors.DefaultSelector()
            sel.register(self.request, selectors.EVENT_READ,
                         self.client_address)
            sel.register(sock, selectors.EVENT_READ, real_server)

            active = True
            while active:
                events = sel.select()
                for key, mask in events:
                    print('\n{} - {} wrote:'.format(datetime.now(), key.data))
                    data = key.fileobj.recv(1024)
                    received = len(data)
                    total += received
                    print(data)
                    print('{} Bytes\nTotal: {} Bytes'.format(received, total))
                    if data:
                        forward[key.fileobj].sendall(data)
                    else:
                        print('\nCLOSING CONNECTION.\n\n')
                        forward[key.fileobj].close()
                        key.fileobj.close()
                        active = False


if __name__ == '__main__':
    p = argparse.ArgumentParser(description='Simple Python Proxy')
    p.add_argument(
        "proxy_port", help="the port where the proxy will listen", type=int)
    p.add_argument('host', help="the real host")
    p.add_argument('port', help="the port of the real host", type=int)
    p.add_argument("--ssl", help="use ssl to connect to the real host",
                   action="store_true")
    args = p.parse_args()

    with Proxy('', args.proxy_port, args.host, args.port, args.ssl) as proxy:
        proxy.serve_forever()
