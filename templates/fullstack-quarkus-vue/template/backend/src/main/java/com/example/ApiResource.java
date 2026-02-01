package com.example;

import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;
import java.util.Map;

@Path("/api")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class ApiResource {

    @GET
    @Path("/hello")
    public Map<String, String> hello() {
        return Map.of("message", "Hello from Quarkus!");
    }

    @GET
    @Path("/health")
    public Map<String, String> health() {
        return Map.of("status", "UP");
    }
}
