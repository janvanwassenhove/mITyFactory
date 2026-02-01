package com.example;

import io.quarkus.test.junit.QuarkusTest;
import org.junit.jupiter.api.Test;

import static io.restassured.RestAssured.given;
import static org.hamcrest.CoreMatchers.is;
import static org.hamcrest.CoreMatchers.notNullValue;

@QuarkusTest
class ApiResourceTest {

    @Test
    void testInfoEndpoint() {
        given()
            .when().get("/api")
            .then()
                .statusCode(200)
                .body("version", is("0.1.0"))
                .body("runtime", is("quarkus"))
                .body("timestamp", notNullValue());
    }

    @Test
    void testHelloEndpoint() {
        given()
            .when().get("/api/hello")
            .then()
                .statusCode(200)
                .body("message", notNullValue());
    }
}
