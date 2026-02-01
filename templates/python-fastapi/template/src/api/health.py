"""Health check endpoints."""

from datetime import datetime

from fastapi import APIRouter
from pydantic import BaseModel

router = APIRouter()


class HealthResponse(BaseModel):
    """Health check response model."""

    status: str
    timestamp: str
    version: str


class ReadinessResponse(BaseModel):
    """Readiness check response model."""

    ready: bool
    checks: dict[str, bool]


@router.get("/health", response_model=HealthResponse)
async def health_check() -> HealthResponse:
    """
    Health check endpoint.
    
    Returns the current health status of the application.
    """
    return HealthResponse(
        status="healthy",
        timestamp=datetime.utcnow().isoformat(),
        version="0.1.0",
    )


@router.get("/ready", response_model=ReadinessResponse)
async def readiness_check() -> ReadinessResponse:
    """
    Readiness check endpoint.
    
    Returns whether the application is ready to serve traffic.
    """
    # Add your readiness checks here (database, cache, external services, etc.)
    checks = {
        "api": True,
        # "database": check_database(),
        # "cache": check_cache(),
    }
    
    return ReadinessResponse(
        ready=all(checks.values()),
        checks=checks,
    )


@router.get("/live")
async def liveness_check() -> dict[str, str]:
    """
    Liveness check endpoint.
    
    Simple endpoint to verify the application is running.
    """
    return {"status": "alive"}
