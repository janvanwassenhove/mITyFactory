using {{project_name}}.Domain.Entities;

namespace {{project_name}}.Domain.Repositories;

/// <summary>
/// Repository interface for Item entities
/// </summary>
public interface IItemRepository
{
    Task<IEnumerable<Item>> GetAllAsync(CancellationToken cancellationToken = default);
    Task<Item?> GetByIdAsync(Guid id, CancellationToken cancellationToken = default);
    Task<Item> CreateAsync(Item item, CancellationToken cancellationToken = default);
    Task<Item?> UpdateAsync(Item item, CancellationToken cancellationToken = default);
    Task<bool> DeleteAsync(Guid id, CancellationToken cancellationToken = default);
}
