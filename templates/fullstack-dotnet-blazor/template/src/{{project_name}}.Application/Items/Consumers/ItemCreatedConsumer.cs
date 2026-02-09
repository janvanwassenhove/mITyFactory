using MassTransit;
using Microsoft.Extensions.Logging;

namespace {{project_name}}.Application.Items.Consumers;

/// <summary>
/// Consumer for ItemCreated events
/// </summary>
public class ItemCreatedConsumer : IConsumer<ItemCreatedEvent>
{
    private readonly ILogger<ItemCreatedConsumer> _logger;

    public ItemCreatedConsumer(ILogger<ItemCreatedConsumer> logger)
    {
        _logger = logger;
    }

    public Task Consume(ConsumeContext<ItemCreatedEvent> context)
    {
        _logger.LogInformation(
            "Processing ItemCreated event: {ItemId} - {ItemName}",
            context.Message.Id,
            context.Message.Name);

        // Add your event handling logic here
        // Examples: send notifications, update caches, trigger workflows
        
        return Task.CompletedTask;
    }
}

/// <summary>
/// Event published when an item is created
/// </summary>
public record ItemCreatedEvent(Guid Id, string Name);
