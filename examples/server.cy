fn main() Int {
    log_set_level(1)
    log_info("starting server on port 8080")
    let server = http_listen(8080)

    let mut count = 0
    while count < 10 {
        let req = server.accept()
        let path = req.path()

        if path == "/" {
            req.respond(200, "Hello from Cyflym!")
        } else {
            req.respond(404, "Not Found")
        }
        count = count + 1
    }
    0
}
