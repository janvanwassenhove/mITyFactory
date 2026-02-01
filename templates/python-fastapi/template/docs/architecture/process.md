# Process View

The Process View describes the system's runtime behavior, async patterns, and concurrency.

## Runtime Architecture

```mermaid
graph TB
    subgraph "Container"
        subgraph "Uvicorn Process"
            LOOP[Event Loop]
            WORKERS[Worker Tasks]
        end
        
        subgraph "FastAPI App"
            ROUTES[Route Handlers]
            MW[Middleware Stack]
            DEPS[Dependency Graph]
        end
    end
    
    REQ[Request] --> LOOP
    LOOP --> MW
    MW --> ROUTES
    ROUTES --> DEPS
    
    style LOOP fill:#e94560,color:#fff
    style ROUTES fill:#16213e
```

## Async Request Flow

```mermaid
sequenceDiagram
    participant Client
    participant Uvicorn
    participant FastAPI
    participant Service
    participant DB
    
    Client->>Uvicorn: HTTP Request
    Uvicorn->>FastAPI: ASGI
    
    activate FastAPI
    FastAPI->>FastAPI: Middleware
    FastAPI->>Service: await handler()
    activate Service
    
    Service->>DB: await query()
    Note right of DB: Non-blocking I/O
    DB-->>Service: Result
    
    Service-->>FastAPI: Response
    deactivate Service
    deactivate FastAPI
    
    FastAPI-->>Client: JSON Response
```

## Concurrency Model

| Component | Model | Notes |
|-----------|-------|-------|
| Uvicorn | asyncio event loop | Single process, async |
| Route handlers | async/await | Non-blocking |
| DB operations | async drivers | asyncpg/aiosqlite |
| External calls | httpx async | Non-blocking HTTP |

## Connection Pools

```mermaid
graph LR
    subgraph "Connection Pools"
        DB[DB Pool<br/>min: 5, max: 20]
        HTTP[HTTP Pool<br/>max: 100]
    end
```

## Middleware Stack

```mermaid
graph TD
    REQ[Request] --> CORS[CORS Middleware]
    CORS --> AUTH[Auth Middleware]
    AUTH --> LOG[Logging Middleware]
    LOG --> ROUTE[Route Handler]
    ROUTE --> RESP[Response]
    
    style REQ fill:#e94560,color:#fff
    style ROUTE fill:#16213e
```

## Error Handling

```mermaid
graph TD
    ERR[Exception] --> EXH[Exception Handler]
    EXH --> LOG[Log Error]
    EXH --> RESP[HTTPException Response]
    RESP --> CLIENT[Client]
    
    style ERR fill:#ef4444,color:#fff
```

---
*Updated by Architect agent on {{date}}*
