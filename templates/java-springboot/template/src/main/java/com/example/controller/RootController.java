package com.example.controller;

import org.springframework.stereotype.Controller;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.ResponseBody;

/**
 * Root controller for the application entry point.
 * Provides a landing page with links to API documentation.
 */
@Controller
public class RootController {

    /**
     * Root endpoint displaying a welcome page with API links.
     */
    @GetMapping("/")
    @ResponseBody
    public String home() {
        return """
            <!DOCTYPE html>
            <html>
            <head>
                <title>{{project_name}}</title>
                <style>
                    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; 
                           max-width: 600px; margin: 50px auto; padding: 20px; }
                    h1 { color: #333; }
                    ul { line-height: 2; }
                    a { color: #0066cc; }
                </style>
            </head>
            <body>
                <h1>🚀 {{project_name}}</h1>
                <p>Spring Boot application is running!</p>
                <h2>Available endpoints:</h2>
                <ul>
                    <li><a href="/api">/api</a> - Service info</li>
                    <li><a href="/api/hello">/api/hello</a> - Hello endpoint</li>
                    <li><a href="/swagger-ui.html">/swagger-ui.html</a> - API Documentation</li>
                    <li><a href="/actuator/health">/actuator/health</a> - Health check</li>
                </ul>
            </body>
            </html>
            """;
    }
}
