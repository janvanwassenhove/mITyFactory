using Bogus;
using FluentAssertions;
using Microsoft.Extensions.Logging;
using NSubstitute;
using {{project_name}}.Application.Items;
using {{project_name}}.Domain.Entities;
using {{project_name}}.Domain.Repositories;
using {{project_name}}.Shared.Models;

namespace {{project_name}}.Tests.Unit.Application;

public class ItemServiceTests
{
    private readonly IItemRepository _repository;
    private readonly ILogger<ItemService> _logger;
    private readonly ItemService _sut;
    private readonly Faker<Item> _itemFaker;

    public ItemServiceTests()
    {
        _repository = Substitute.For<IItemRepository>();
        _logger = Substitute.For<ILogger<ItemService>>();
        _sut = new ItemService(_repository, _logger);
        
        _itemFaker = new Faker<Item>()
            .RuleFor(x => x.Id, f => f.Random.Guid())
            .RuleFor(x => x.Name, f => f.Commerce.ProductName())
            .RuleFor(x => x.Description, f => f.Commerce.ProductDescription())
            .RuleFor(x => x.IsActive, f => f.Random.Bool())
            .RuleFor(x => x.CreatedAt, f => f.Date.Past());
    }

    [Fact]
    public async Task GetAllAsync_ShouldReturnAllItems()
    {
        // Arrange
        var items = _itemFaker.Generate(5);
        _repository.GetAllAsync(Arg.Any<CancellationToken>()).Returns(items);

        // Act
        var result = await _sut.GetAllAsync();

        // Assert
        result.Should().HaveCount(5);
        await _repository.Received(1).GetAllAsync(Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task GetByIdAsync_WhenItemExists_ShouldReturnItem()
    {
        // Arrange
        var item = _itemFaker.Generate();
        _repository.GetByIdAsync(item.Id, Arg.Any<CancellationToken>()).Returns(item);

        // Act
        var result = await _sut.GetByIdAsync(item.Id);

        // Assert
        result.Should().NotBeNull();
        result!.Id.Should().Be(item.Id);
        result.Name.Should().Be(item.Name);
    }

    [Fact]
    public async Task GetByIdAsync_WhenItemNotFound_ShouldReturnNull()
    {
        // Arrange
        var id = Guid.NewGuid();
        _repository.GetByIdAsync(id, Arg.Any<CancellationToken>()).Returns((Item?)null);

        // Act
        var result = await _sut.GetByIdAsync(id);

        // Assert
        result.Should().BeNull();
    }

    [Fact]
    public async Task CreateAsync_ShouldCreateAndReturnItem()
    {
        // Arrange
        var request = new CreateItemRequest { Name = "Test Item", Description = "Test Description" };
        _repository.CreateAsync(Arg.Any<Item>(), Arg.Any<CancellationToken>())
            .Returns(callInfo => callInfo.Arg<Item>());

        // Act
        var result = await _sut.CreateAsync(request);

        // Assert
        result.Should().NotBeNull();
        result.Name.Should().Be(request.Name);
        result.Description.Should().Be(request.Description);
        result.IsActive.Should().BeTrue();
        await _repository.Received(1).CreateAsync(Arg.Any<Item>(), Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task UpdateAsync_WhenItemExists_ShouldUpdateAndReturnItem()
    {
        // Arrange
        var existingItem = _itemFaker.Generate();
        var request = new UpdateItemRequest { Name = "Updated Name" };
        
        _repository.GetByIdAsync(existingItem.Id, Arg.Any<CancellationToken>()).Returns(existingItem);
        _repository.UpdateAsync(Arg.Any<Item>(), Arg.Any<CancellationToken>())
            .Returns(callInfo => callInfo.Arg<Item>());

        // Act
        var result = await _sut.UpdateAsync(existingItem.Id, request);

        // Assert
        result.Should().NotBeNull();
        result!.Name.Should().Be("Updated Name");
        await _repository.Received(1).UpdateAsync(Arg.Any<Item>(), Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task UpdateAsync_WhenItemNotFound_ShouldReturnNull()
    {
        // Arrange
        var id = Guid.NewGuid();
        var request = new UpdateItemRequest { Name = "Updated Name" };
        _repository.GetByIdAsync(id, Arg.Any<CancellationToken>()).Returns((Item?)null);

        // Act
        var result = await _sut.UpdateAsync(id, request);

        // Assert
        result.Should().BeNull();
        await _repository.DidNotReceive().UpdateAsync(Arg.Any<Item>(), Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task DeleteAsync_WhenItemExists_ShouldReturnTrue()
    {
        // Arrange
        var id = Guid.NewGuid();
        _repository.DeleteAsync(id, Arg.Any<CancellationToken>()).Returns(true);

        // Act
        var result = await _sut.DeleteAsync(id);

        // Assert
        result.Should().BeTrue();
        await _repository.Received(1).DeleteAsync(id, Arg.Any<CancellationToken>());
    }

    [Fact]
    public async Task DeleteAsync_WhenItemNotFound_ShouldReturnFalse()
    {
        // Arrange
        var id = Guid.NewGuid();
        _repository.DeleteAsync(id, Arg.Any<CancellationToken>()).Returns(false);

        // Act
        var result = await _sut.DeleteAsync(id);

        // Assert
        result.Should().BeFalse();
    }
}
