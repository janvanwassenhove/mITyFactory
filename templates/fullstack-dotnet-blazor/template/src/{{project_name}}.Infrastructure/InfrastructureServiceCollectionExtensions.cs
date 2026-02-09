using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using {{project_name}}.Domain.Repositories;
using {{project_name}}.Infrastructure.Data;
using {{project_name}}.Infrastructure.Repositories;

namespace {{project_name}}.Infrastructure;

/// <summary>
/// Extension methods for registering infrastructure services
/// </summary>
public static class InfrastructureServiceCollectionExtensions
{
    public static IServiceCollection AddInfrastructureServices(
        this IServiceCollection services,
        IConfiguration configuration)
    {
        // Add DbContext
        services.AddDbContext<ApplicationDbContext>(options =>
            options.UseNpgsql(
                configuration.GetConnectionString("Database"),
                b => b.MigrationsAssembly(typeof(ApplicationDbContext).Assembly.FullName)));

        // Add repositories
        services.AddScoped<IItemRepository, ItemRepository>();

        return services;
    }
}
