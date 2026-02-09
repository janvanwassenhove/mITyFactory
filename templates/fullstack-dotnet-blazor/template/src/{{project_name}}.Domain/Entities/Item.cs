namespace {{project_name}}.Domain.Entities;

/// <summary>
/// Item entity
/// </summary>
public class Item : BaseEntity
{
    public string Name { get; set; } = string.Empty;
    public string? Description { get; set; }
    public bool IsActive { get; set; } = true;
}
