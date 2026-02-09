using Microsoft.EntityFrameworkCore;
using {{project_name}}.Domain.Entities;
using {{project_name}}.Domain.Repositories;
using {{project_name}}.Infrastructure.Data;

namespace {{project_name}}.Infrastructure.Repositories;

/// <summary>
/// Repository implementation for Item entities
/// </summary>
public class ItemRepository : IItemRepository
{
    private readonly ApplicationDbContext _context;

    public ItemRepository(ApplicationDbContext context)
    {
        _context = context;
    }

    public async Task<IEnumerable<Item>> GetAllAsync(CancellationToken cancellationToken = default)
    {
        return await _context.Items
            .AsNoTracking()
            .OrderByDescending(x => x.CreatedAt)
            .ToListAsync(cancellationToken);
    }

    public async Task<Item?> GetByIdAsync(Guid id, CancellationToken cancellationToken = default)
    {
        return await _context.Items
            .FirstOrDefaultAsync(x => x.Id == id, cancellationToken);
    }

    public async Task<Item> CreateAsync(Item item, CancellationToken cancellationToken = default)
    {
        _context.Items.Add(item);
        await _context.SaveChangesAsync(cancellationToken);
        return item;
    }

    public async Task<Item?> UpdateAsync(Item item, CancellationToken cancellationToken = default)
    {
        _context.Items.Update(item);
        await _context.SaveChangesAsync(cancellationToken);
        return item;
    }

    public async Task<bool> DeleteAsync(Guid id, CancellationToken cancellationToken = default)
    {
        var item = await _context.Items.FindAsync(new object[] { id }, cancellationToken);
        
        if (item is null)
            return false;

        _context.Items.Remove(item);
        await _context.SaveChangesAsync(cancellationToken);
        return true;
    }
}
