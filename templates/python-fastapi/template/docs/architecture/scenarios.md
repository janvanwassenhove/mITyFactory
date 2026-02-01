# Scenarios View (+1)

The Scenarios view captures the key use cases and user journeys that drive architectural decisions.

## System Context

```mermaid
graph LR
    subgraph Users
        U1[End User]
        U2[Admin]
    end
    
    subgraph "{{app_name}}"
        API[FastAPI Service]
    end
    
    subgraph External
        DB[(Database)]
        EXT[External APIs]
    end
    
    U1 -->|HTTP| API
    U2 -->|HTTP| API
    API -->|async| DB
    API -->|HTTP| EXT
    
    style API fill:#e94560,color:#fff
```

## Primary Use Cases

### UC-1: [Primary Use Case Name]
**Actor:** End User  
**Description:** [Brief description of the use case]

```mermaid
sequenceDiagram
    actor User
    participant API as FastAPI
    participant Service
    participant DB
    
    User->>API: HTTP Request
    API->>Service: await process()
    Service->>DB: await query()
    DB-->>Service: Result
    Service-->>API: Response Model
    API-->>User: JSON Response
```

## Quality Attribute Scenarios

| ID | Quality Attribute | Scenario | Response Measure |
|----|------------------|----------|------------------|
| QA-1 | Performance | Async I/O bound operations | < 100ms response |
| QA-2 | Availability | System failure | 99.9% uptime |
| QA-3 | Security | Unauthorized access | OAuth2/JWT auth |

---
*Updated by Architect agent on {{date}}*
