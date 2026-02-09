using System.Net;
using System.Net.Http.Json;
using FluentAssertions;
using {{project_name}}.Shared.Models;

namespace {{project_name}}.Tests.Integration.Endpoints;

public class ItemEndpointsTests : IClassFixture<IntegrationTestFactory>
{
    private readonly HttpClient _client;

    public ItemEndpointsTests(IntegrationTestFactory factory)
    {
        _client = factory.CreateClient();
    }

    [Fact]
    public async Task GetAllItems_WhenEmpty_ReturnsEmptyList()
    {
        // Act
        var response = await _client.GetAsync("/api/v1/items");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var items = await response.Content.ReadFromJsonAsync<List<ItemDto>>();
        items.Should().NotBeNull();
    }

    [Fact]
    public async Task CreateItem_WithValidData_ReturnsCreatedItem()
    {
        // Arrange
        var request = new { Name = "Test Item", Description = "Test Description" };

        // Act
        var response = await _client.PostAsJsonAsync("/api/v1/items", request);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Created);
        var item = await response.Content.ReadFromJsonAsync<ItemDto>();
        item.Should().NotBeNull();
        item!.Name.Should().Be("Test Item");
        item.Description.Should().Be("Test Description");
        item.IsActive.Should().BeTrue();
        item.Id.Should().NotBeEmpty();
    }

    [Fact]
    public async Task GetItemById_WhenExists_ReturnsItem()
    {
        // Arrange - Create an item first
        var createRequest = new { Name = "Get By Id Test", Description = "Description" };
        var createResponse = await _client.PostAsJsonAsync("/api/v1/items", createRequest);
        var createdItem = await createResponse.Content.ReadFromJsonAsync<ItemDto>();

        // Act
        var response = await _client.GetAsync($"/api/v1/items/{createdItem!.Id}");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var item = await response.Content.ReadFromJsonAsync<ItemDto>();
        item.Should().NotBeNull();
        item!.Id.Should().Be(createdItem.Id);
        item.Name.Should().Be("Get By Id Test");
    }

    [Fact]
    public async Task GetItemById_WhenNotExists_ReturnsNotFound()
    {
        // Arrange
        var nonExistentId = Guid.NewGuid();

        // Act
        var response = await _client.GetAsync($"/api/v1/items/{nonExistentId}");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task UpdateItem_WhenExists_ReturnsUpdatedItem()
    {
        // Arrange - Create an item first
        var createRequest = new { Name = "Original Name", Description = "Original" };
        var createResponse = await _client.PostAsJsonAsync("/api/v1/items", createRequest);
        var createdItem = await createResponse.Content.ReadFromJsonAsync<ItemDto>();

        var updateRequest = new { Name = "Updated Name", Description = "Updated" };

        // Act
        var response = await _client.PutAsJsonAsync($"/api/v1/items/{createdItem!.Id}", updateRequest);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var item = await response.Content.ReadFromJsonAsync<ItemDto>();
        item.Should().NotBeNull();
        item!.Name.Should().Be("Updated Name");
        item.Description.Should().Be("Updated");
    }

    [Fact]
    public async Task DeleteItem_WhenExists_ReturnsNoContent()
    {
        // Arrange - Create an item first
        var createRequest = new { Name = "To Delete", Description = "Will be deleted" };
        var createResponse = await _client.PostAsJsonAsync("/api/v1/items", createRequest);
        var createdItem = await createResponse.Content.ReadFromJsonAsync<ItemDto>();

        // Act
        var response = await _client.DeleteAsync($"/api/v1/items/{createdItem!.Id}");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NoContent);

        // Verify item is deleted
        var getResponse = await _client.GetAsync($"/api/v1/items/{createdItem.Id}");
        getResponse.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task DeleteItem_WhenNotExists_ReturnsNotFound()
    {
        // Arrange
        var nonExistentId = Guid.NewGuid();

        // Act
        var response = await _client.DeleteAsync($"/api/v1/items/{nonExistentId}");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task HealthCheck_ReturnsHealthy()
    {
        // Act
        var response = await _client.GetAsync("/health/live");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
    }
}
