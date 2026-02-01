"""API route definitions."""

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from src.models.schemas import Item, ItemCreate, ItemList
from src.services.example import ExampleService

router = APIRouter()
service = ExampleService()


@router.get("/", response_model=dict[str, str])
async def root() -> dict[str, str]:
    """Root endpoint."""
    return {"message": "Welcome to {{project_name}} API"}


@router.get("/items", response_model=ItemList)
async def list_items() -> ItemList:
    """List all items."""
    items = await service.list_items()
    return ItemList(items=items, total=len(items))


@router.get("/items/{item_id}", response_model=Item)
async def get_item(item_id: str) -> Item:
    """Get a specific item by ID."""
    item = await service.get_item(item_id)
    if item is None:
        raise HTTPException(status_code=404, detail="Item not found")
    return item


@router.post("/items", response_model=Item, status_code=201)
async def create_item(item: ItemCreate) -> Item:
    """Create a new item."""
    return await service.create_item(item)


@router.delete("/items/{item_id}", status_code=204)
async def delete_item(item_id: str) -> None:
    """Delete an item."""
    deleted = await service.delete_item(item_id)
    if not deleted:
        raise HTTPException(status_code=404, detail="Item not found")
