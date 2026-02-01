package com.example;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;

import java.time.Instant;
import java.util.Map;

/**
 * Main API resource for the Quarkus application.
 */
@Path("/api")
@Produces(MediaType.APPLICATION_JSON)
public class ApiResource {

    /**
     * Root endpoint returning service info.
     */
    @GET
    public Map<String, Object> getInfo() {
        return Map.of(
            "service", "{{project_name}}",
            "version", "0.1.0",
            "runtime", "quarkus",
            "timestamp", Instant.now().toString()
        );
    }

    /**
     * Hello endpoint.
     */
    @GET
    @Path("/hello")
    public Map<String, String> hello() {
        return Map.of(
            "message", "Hello from {{project_name}} on Quarkus!"
        );
    }
}
