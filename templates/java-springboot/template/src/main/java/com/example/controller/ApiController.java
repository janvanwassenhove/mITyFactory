package com.example.controller;

import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.time.Instant;
import java.util.Map;

/**
 * Sample REST controller demonstrating API patterns.
 */
@RestController
@RequestMapping("/api")
public class ApiController {

    /**
     * Root endpoint returning service info.
     */
    @GetMapping
    public ResponseEntity<Map<String, Object>> getInfo() {
        return ResponseEntity.ok(Map.of(
            "service", "{{project_name}}",
            "version", "0.1.0",
            "timestamp", Instant.now().toString()
        ));
    }

    /**
     * Hello endpoint.
     */
    @GetMapping("/hello")
    public ResponseEntity<Map<String, String>> hello() {
        return ResponseEntity.ok(Map.of(
            "message", "Hello from {{project_name}}!"
        ));
    }
}
