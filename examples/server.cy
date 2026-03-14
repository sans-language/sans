fn main() Int {
    log_set_level(1)
    let port = 8080
    log_info("starting server on port 8080")
    let server = http_listen(port)

    // Handle 3 requests then exit
    let mut count = 0
    while count < 3 {
        let req = server.accept()
        let path = req.path()
        let method = req.method()
        log_info("request: " + method + " " + path)

        if path == "/" {
            req.respond(200, "Hello from Cyflym!")
        } else {
            if path == "/json" {
                let obj = json_object()
                obj.set("message", json_string("hello"))
                obj.set("count", json_int(count))
                let body = json_stringify(obj)
                req.respond(200, body)
            } else {
                req.respond(404, "Not Found")
            }
        }
        count = count + 1
    }

    log_info("server shutting down")
    0
}
