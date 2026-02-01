"""Unit tests for health endpoints."""

import pytest
from fastapi.testclient import TestClient

from src.main import app


@pytest.fixture
def client() -> TestClient:
    """Create a test client."""
    return TestClient(app)


class TestHealthEndpoints:
    """Tests for health check endpoints."""

    def test_health_check(self, client: TestClient) -> None:
        """Test the health check endpoint returns healthy status."""
        response = client.get("/health")
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "healthy"
        assert "timestamp" in data
        assert "version" in data

    def test_readiness_check(self, client: TestClient) -> None:
        """Test the readiness check endpoint."""
        response = client.get("/ready")
        
        assert response.status_code == 200
        data = response.json()
        assert "ready" in data
        assert "checks" in data
        assert data["ready"] is True

    def test_liveness_check(self, client: TestClient) -> None:
        """Test the liveness check endpoint."""
        response = client.get("/live")
        
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "alive"


class TestApiEndpoints:
    """Tests for API endpoints."""

    def test_root_endpoint(self, client: TestClient) -> None:
        """Test the root API endpoint."""
        response = client.get("/api/v1/")
        
        assert response.status_code == 200
        data = response.json()
        assert "message" in data

    def test_list_items_empty(self, client: TestClient) -> None:
        """Test listing items when empty."""
        response = client.get("/api/v1/items")
        
        assert response.status_code == 200
        data = response.json()
        assert data["items"] == []
        assert data["total"] == 0

    def test_create_item(self, client: TestClient) -> None:
        """Test creating a new item."""
        item_data = {
            "name": "Test Item",
            "description": "A test item",
            "tags": ["test", "example"],
        }
        
        response = client.post("/api/v1/items", json=item_data)
        
        assert response.status_code == 201
        data = response.json()
        assert data["name"] == item_data["name"]
        assert data["description"] == item_data["description"]
        assert data["tags"] == item_data["tags"]
        assert "id" in data
        assert "created_at" in data

    def test_get_item_not_found(self, client: TestClient) -> None:
        """Test getting a non-existent item."""
        response = client.get("/api/v1/items/nonexistent")
        
        assert response.status_code == 404

    def test_delete_item_not_found(self, client: TestClient) -> None:
        """Test deleting a non-existent item."""
        response = client.delete("/api/v1/items/nonexistent")
        
        assert response.status_code == 404
