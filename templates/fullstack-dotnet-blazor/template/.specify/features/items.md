# Item Management Feature

## Overview

Basic CRUD operations for Items with messaging support.

## User Stories

### US-001: List Items

**As a** user  
**I want to** view a list of all items  
**So that** I can see what items exist in the system

**Acceptance Criteria:**
- Display all items in a grid layout
- Show item name, description, status, and creation date
- Items ordered by creation date (newest first)

### US-002: Create Item

**As a** user  
**I want to** create a new item  
**So that** I can add items to the system

**Acceptance Criteria:**
- Modal form with name and description fields
- Name is required
- Item created with active status
- Publish ItemCreated event via MassTransit

### US-003: Update Item

**As a** user  
**I want to** update an existing item  
**So that** I can modify item details

**Acceptance Criteria:**
- Can update name, description, and active status
- Only changed fields are updated
- UpdatedAt timestamp set on modification

### US-004: Delete Item

**As a** user  
**I want to** delete an item  
**So that** I can remove items from the system

**Acceptance Criteria:**
- Hard delete from database
- Publish ItemDeleted event via MassTransit

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/v1/items | List all items |
| GET | /api/v1/items/{id} | Get item by ID |
| POST | /api/v1/items | Create new item |
| PUT | /api/v1/items/{id} | Update item |
| DELETE | /api/v1/items/{id} | Delete item |

## Events

- `ItemCreatedEvent`: Published when an item is created
- `ItemDeletedEvent`: Published when an item is deleted
