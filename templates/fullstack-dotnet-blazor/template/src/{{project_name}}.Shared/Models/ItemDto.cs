namespace {{project_name}}.Shared.Models;

/// <summary>
/// Data transfer object for Item
/// </summary>
public record ItemDto
{
    public Guid Id { get; init; }
    public string Name { get; init; } = string.Empty;
    public string? Description { get; init; }
    public bool IsActive { get; init; }
    public DateTime CreatedAt { get; init; }
    public DateTime? UpdatedAt { get; init; }
}

/// <summary>
/// Request to create a new item
/// </summary>
public record CreateItemRequest
{
    public required string Name { get; init; }
    public string? Description { get; init; }
}

/// <summary>
/// Request to update an existing item
/// </summary>
public record UpdateItemRequest
{
    public string? Name { get; init; }
    public string? Description { get; init; }
    public bool? IsActive { get; init; }
}
