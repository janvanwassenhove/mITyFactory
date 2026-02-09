using Microsoft.Extensions.DependencyInjection;

namespace {{project_name}}.Shared;

/// <summary>
/// Extension methods for registering shared services
/// </summary>
public static class SharedServiceCollectionExtensions
{
    public static IServiceCollection AddSharedServices(this IServiceCollection services)
    {
        // Add any shared services here
        return services;
    }
}
