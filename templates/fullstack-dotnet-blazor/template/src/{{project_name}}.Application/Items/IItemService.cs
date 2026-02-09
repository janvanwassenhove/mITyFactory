using {{project_name}}.Shared.Models;

namespace {{project_name}}.Application.Items;

/// <summary>
/// Service interface for Item operations
/// </summary>
public interface IItemService
{
    Task<IEnumerable<ItemDto>> GetAllAsync(CancellationToken cancellationToken = default);
    Task<ItemDto?> GetByIdAsync(Guid id, CancellationToken cancellationToken = default);
    Task<ItemDto> CreateAsync(CreateItemRequest request, CancellationToken cancellationToken = default);
    Task<ItemDto?> UpdateAsync(Guid id, UpdateItemRequest request, CancellationToken cancellationToken = default);
    Task<bool> DeleteAsync(Guid id, CancellationToken cancellationToken = default);
}
