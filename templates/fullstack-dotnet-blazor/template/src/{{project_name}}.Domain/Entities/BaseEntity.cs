namespace {{project_name}}.Domain.Entities;

/// <summary>
/// Base entity with common properties
/// </summary>
public abstract class BaseEntity
{
    public Guid Id { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime? UpdatedAt { get; set; }
}
