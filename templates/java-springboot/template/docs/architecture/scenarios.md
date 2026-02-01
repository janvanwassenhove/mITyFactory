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
        API[REST API]
    end
    
    subgraph External
        DB[(Database)]
        EXT[External Services]
    end
    
    U1 -->|HTTP| API
    U2 -->|HTTP| API
    API -->|JDBC| DB
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
    participant API
    participant Service
    participant DB
    
    User->>API: Request
    API->>Service: Process
    Service->>DB: Query/Update
    DB-->>Service: Result
    Service-->>API: Response
    API-->>User: HTTP Response
```

### UC-2: [Secondary Use Case Name]
**Actor:** Admin  
**Description:** [Brief description]

## Quality Attribute Scenarios

| ID | Quality Attribute | Scenario | Response Measure |
|----|------------------|----------|------------------|
| QA-1 | Performance | Under normal load | < 200ms response |
| QA-2 | Availability | System failure | 99.9% uptime |
| QA-3 | Security | Unauthorized access | All requests authenticated |

---
*Updated by Architect agent on {{date}}*
