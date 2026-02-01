"""Example service implementation."""

import uuid
from datetime import datetime
from typing import Optional

import structlog

from src.models.schemas import Item, ItemCreate

logger = structlog.get_logger(__name__)


class ExampleService:
    """Example service for CRUD operations."""

    def __init__(self) -> None:
        """Initialize the service with in-memory storage."""
        self._items: dict[str, Item] = {}

    async def list_items(self) -> list[Item]:
        """
        List all items.
        
        Returns:
            List of all items.
        """
        logger.info("list_items", count=len(self._items))
        return list(self._items.values())

    async def get_item(self, item_id: str) -> Optional[Item]:
        """
        Get an item by ID.
        
        Args:
            item_id: The item identifier.
            
        Returns:
            The item if found, None otherwise.
        """
        logger.info("get_item", item_id=item_id)
        return self._items.get(item_id)

    async def create_item(self, item_create: ItemCreate) -> Item:
        """
        Create a new item.
        
        Args:
            item_create: The item creation data.
            
        Returns:
            The created item.
        """
        item_id = str(uuid.uuid4())
        now = datetime.utcnow()
        
        item = Item(
            id=item_id,
            name=item_create.name,
            description=item_create.description,
            tags=item_create.tags,
            created_at=now,
            updated_at=None,
        )
        
        self._items[item_id] = item
        logger.info("create_item", item_id=item_id, name=item.name)
        
        return item

    async def update_item(self, item_id: str, item_update: ItemCreate) -> Optional[Item]:
        """
        Update an existing item.
        
        Args:
            item_id: The item identifier.
            item_update: The item update data.
            
        Returns:
            The updated item if found, None otherwise.
        """
        if item_id not in self._items:
            return None
        
        existing = self._items[item_id]
        updated = Item(
            id=item_id,
            name=item_update.name,
            description=item_update.description,
            tags=item_update.tags,
            created_at=existing.created_at,
            updated_at=datetime.utcnow(),
        )
        
        self._items[item_id] = updated
        logger.info("update_item", item_id=item_id)
        
        return updated

    async def delete_item(self, item_id: str) -> bool:
        """
        Delete an item.
        
        Args:
            item_id: The item identifier.
            
        Returns:
            True if deleted, False if not found.
        """
        if item_id in self._items:
            del self._items[item_id]
            logger.info("delete_item", item_id=item_id)
            return True
        return False
