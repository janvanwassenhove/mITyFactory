"""Pydantic models for request/response schemas."""

from datetime import datetime
from typing import Optional

from pydantic import BaseModel, Field


class ItemBase(BaseModel):
    """Base model for items."""

    name: str = Field(..., min_length=1, max_length=100, description="Item name")
    description: Optional[str] = Field(None, max_length=500, description="Item description")
    tags: list[str] = Field(default_factory=list, description="Item tags")


class ItemCreate(ItemBase):
    """Model for creating items."""

    pass


class Item(ItemBase):
    """Model for item responses."""

    id: str = Field(..., description="Unique item identifier")
    created_at: datetime = Field(..., description="Creation timestamp")
    updated_at: Optional[datetime] = Field(None, description="Last update timestamp")

    class Config:
        """Pydantic configuration."""

        from_attributes = True


class ItemList(BaseModel):
    """Model for item list responses."""

    items: list[Item]
    total: int


class ErrorResponse(BaseModel):
    """Model for error responses."""

    detail: str
    code: Optional[str] = None
