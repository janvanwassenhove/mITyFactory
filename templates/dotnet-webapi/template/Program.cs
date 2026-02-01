using Microsoft.AspNetCore.Diagnostics.HealthChecks;
using Microsoft.Extensions.Diagnostics.HealthChecks;

var builder = WebApplication.CreateBuilder(args);

// Add services to the container
builder.Services.AddEndpointsApiExplorer();
builder.Services.AddSwaggerGen(c =>
{
    c.SwaggerDoc("v1", new() { Title = "{{project_name}}", Version = "v1" });
});
builder.Services.AddHealthChecks();

var app = builder.Build();

// Configure the HTTP request pipeline
if (app.Environment.IsDevelopment())
{
    app.UseSwagger();
    app.UseSwaggerUI();
}

// Health check endpoint
app.MapHealthChecks("/health", new HealthCheckOptions
{
    ResponseWriter = async (context, report) =>
    {
        context.Response.ContentType = "application/json";
        var response = new
        {
            status = report.Status.ToString(),
            checks = report.Entries.Select(e => new
            {
                name = e.Key,
                status = e.Value.Status.ToString(),
                description = e.Value.Description
            })
        };
        await context.Response.WriteAsJsonAsync(response);
    }
});

// API endpoints
var api = app.MapGroup("/api");

api.MapGet("/", () => new
{
    Service = "{{project_name}}",
    Version = "0.1.0",
    Timestamp = DateTime.UtcNow
})
.WithName("GetInfo")
.WithOpenApi();

api.MapGet("/hello", () => new { Message = "Hello from {{project_name}}!" })
.WithName("Hello")
.WithOpenApi();

api.MapGet("/hello/{name}", (string name) => new { Message = $"Hello, {name}!" })
.WithName("HelloName")
.WithOpenApi();

app.Run();
