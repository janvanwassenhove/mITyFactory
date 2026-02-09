using Microsoft.Extensions.DependencyInjection;
using {{project_name}}.Application.Items;

namespace {{project_name}}.Application;

/// <summary>
/// Extension methods for registering application services
/// </summary>
public static class ApplicationServiceCollectionExtensions
{
    public static IServiceCollection AddApplicationServices(this IServiceCollection services)
    {
        services.AddScoped<IItemService, ItemService>();
        
        return services;
    }
}
