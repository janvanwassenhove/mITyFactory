using {{project_name}}.Domain.Entities;
using {{project_name}}.Domain.Repositories;
using {{project_name}}.Shared.Models;
using Microsoft.Extensions.Logging;

namespace {{project_name}}.Application.Items;

/// <summary>
/// Service implementation for Item operations
/// </summary>
public class ItemService : IItemService
{
    private readonly IItemRepository _repository;
    private readonly ILogger<ItemService> _logger;

    public ItemService(IItemRepository repository, ILogger<ItemService> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    public async Task<IEnumerable<ItemDto>> GetAllAsync(CancellationToken cancellationToken = default)
    {
        _logger.LogInformation("Getting all items");
        
        var items = await _repository.GetAllAsync(cancellationToken);
        
        return items.Select(MapToDto);
    }

    public async Task<ItemDto?> GetByIdAsync(Guid id, CancellationToken cancellationToken = default)
    {
        _logger.LogInformation("Getting item with ID {ItemId}", id);
        
        var item = await _repository.GetByIdAsync(id, cancellationToken);
        
        return item is null ? null : MapToDto(item);
    }

    public async Task<ItemDto> CreateAsync(CreateItemRequest request, CancellationToken cancellationToken = default)
    {
        _logger.LogInformation("Creating new item: {ItemName}", request.Name);
        
        var item = new Item
        {
            Id = Guid.NewGuid(),
            Name = request.Name,
            Description = request.Description,
            IsActive = true,
            CreatedAt = DateTime.UtcNow
        };

        var created = await _repository.CreateAsync(item, cancellationToken);
        
        _logger.LogInformation("Created item with ID {ItemId}", created.Id);
        
        return MapToDto(created);
    }

    public async Task<ItemDto?> UpdateAsync(Guid id, UpdateItemRequest request, CancellationToken cancellationToken = default)
    {
        _logger.LogInformation("Updating item with ID {ItemId}", id);
        
        var existing = await _repository.GetByIdAsync(id, cancellationToken);
        
        if (existing is null)
        {
            _logger.LogWarning("Item with ID {ItemId} not found", id);
            return null;
        }

        if (request.Name is not null)
            existing.Name = request.Name;
        
        if (request.Description is not null)
            existing.Description = request.Description;
        
        if (request.IsActive.HasValue)
            existing.IsActive = request.IsActive.Value;

        existing.UpdatedAt = DateTime.UtcNow;

        var updated = await _repository.UpdateAsync(existing, cancellationToken);
        
        return updated is null ? null : MapToDto(updated);
    }

    public async Task<bool> DeleteAsync(Guid id, CancellationToken cancellationToken = default)
    {
        _logger.LogInformation("Deleting item with ID {ItemId}", id);
        
        return await _repository.DeleteAsync(id, cancellationToken);
    }

    private static ItemDto MapToDto(Item item) => new()
    {
        Id = item.Id,
        Name = item.Name,
        Description = item.Description,
        IsActive = item.IsActive,
        CreatedAt = item.CreatedAt,
        UpdatedAt = item.UpdatedAt
    };
}
