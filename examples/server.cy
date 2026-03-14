fn main() Int {
    log_set_level(0)
    print("Starting server on http://localhost:8080")
    let server = http_listen(8080)

    let mut count = 0
    while count < 20 {
        log_debug("waiting for request...")
        let req = server.accept()
        let path = req.path()
        log_info("request: " + path)

        if path == "/" {
            req.respond(200, "Hello from Cyflym!")
        } else {
            req.respond(404, "Not Found")
        }
        count = count + 1
    }
    0
}
