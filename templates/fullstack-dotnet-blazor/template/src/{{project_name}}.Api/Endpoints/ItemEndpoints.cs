using {{project_name}}.Application.Items;
using {{project_name}}.Shared.Models;
using MassTransit;
using Microsoft.AspNetCore.Http.HttpResults;

namespace {{project_name}}.Api.Endpoints;

/// <summary>
/// Item management endpoints
/// </summary>
public static class ItemEndpoints
{
    public static void MapItemEndpoints(this IEndpointRouteBuilder routes)
    {
        var group = routes.MapGroup("/api/v1/items")
            .WithTags("Items")
            .WithOpenApi();

        group.MapGet("/", GetAllItems)
            .WithName("GetAllItems")
            .WithSummary("Get all items")
            .Produces<List<ItemDto>>();

        group.MapGet("/{id:guid}", GetItemById)
            .WithName("GetItemById")
            .WithSummary("Get item by ID")
            .Produces<ItemDto>()
            .Produces(StatusCodes.Status404NotFound);

        group.MapPost("/", CreateItem)
            .WithName("CreateItem")
            .WithSummary("Create a new item")
            .Produces<ItemDto>(StatusCodes.Status201Created)
            .Produces<ValidationProblem>(StatusCodes.Status400BadRequest);

        group.MapPut("/{id:guid}", UpdateItem)
            .WithName("UpdateItem")
            .WithSummary("Update an existing item")
            .Produces<ItemDto>()
            .Produces(StatusCodes.Status404NotFound);

        group.MapDelete("/{id:guid}", DeleteItem)
            .WithName("DeleteItem")
            .WithSummary("Delete an item")
            .Produces(StatusCodes.Status204NoContent)
            .Produces(StatusCodes.Status404NotFound);
    }

    private static async Task<Ok<List<ItemDto>>> GetAllItems(
        IItemService itemService,
        CancellationToken cancellationToken)
    {
        var items = await itemService.GetAllAsync(cancellationToken);
        return TypedResults.Ok(items.ToList());
    }

    private static async Task<Results<Ok<ItemDto>, NotFound>> GetItemById(
        Guid id,
        IItemService itemService,
        CancellationToken cancellationToken)
    {
        var item = await itemService.GetByIdAsync(id, cancellationToken);
        
        return item is not null
            ? TypedResults.Ok(item)
            : TypedResults.NotFound();
    }

    private static async Task<Results<Created<ItemDto>, ValidationProblem>> CreateItem(
        CreateItemRequest request,
        IItemService itemService,
        IPublishEndpoint publishEndpoint,
        CancellationToken cancellationToken)
    {
        var item = await itemService.CreateAsync(request, cancellationToken);
        
        // Publish event via MassTransit
        await publishEndpoint.Publish(new ItemCreatedEvent(item.Id, item.Name), cancellationToken);
        
        return TypedResults.Created($"/api/v1/items/{item.Id}", item);
    }

    private static async Task<Results<Ok<ItemDto>, NotFound>> UpdateItem(
        Guid id,
        UpdateItemRequest request,
        IItemService itemService,
        CancellationToken cancellationToken)
    {
        var item = await itemService.UpdateAsync(id, request, cancellationToken);
        
        return item is not null
            ? TypedResults.Ok(item)
            : TypedResults.NotFound();
    }

    private static async Task<Results<NoContent, NotFound>> DeleteItem(
        Guid id,
        IItemService itemService,
        IPublishEndpoint publishEndpoint,
        CancellationToken cancellationToken)
    {
        var deleted = await itemService.DeleteAsync(id, cancellationToken);
        
        if (deleted)
        {
            await publishEndpoint.Publish(new ItemDeletedEvent(id), cancellationToken);
            return TypedResults.NoContent();
        }
        
        return TypedResults.NotFound();
    }
}

// Events for MassTransit
public record ItemCreatedEvent(Guid Id, string Name);
public record ItemDeletedEvent(Guid Id);
