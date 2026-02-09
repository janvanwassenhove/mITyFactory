using {{project_name}}.Api.Endpoints;
using {{project_name}}.Application;
using {{project_name}}.Infrastructure;
using {{project_name}}.Shared;
using FluentValidation;
using HealthChecks.UI.Client;
using MassTransit;
using Microsoft.AspNetCore.Diagnostics.HealthChecks;
using OpenTelemetry.Metrics;
using OpenTelemetry.Resources;
using OpenTelemetry.Trace;
using Scalar.AspNetCore;
using Serilog;

// Bootstrap Serilog for startup logging
Log.Logger = new LoggerConfiguration()
    .WriteTo.Console()
    .CreateBootstrapLogger();

try
{
    Log.Information("Starting {{project_name}}.Api");

    var builder = WebApplication.CreateBuilder(args);

    // Configure Serilog
    builder.Host.UseSerilog((context, services, configuration) => configuration
        .ReadFrom.Configuration(context.Configuration)
        .ReadFrom.Services(services)
        .Enrich.FromLogContext()
        .Enrich.WithEnvironmentName()
        .Enrich.WithThreadId()
        .WriteTo.Console(
            outputTemplate: "[{Timestamp:HH:mm:ss} {Level:u3}] {Message:lj} {Properties:j}{NewLine}{Exception}"));

    // Add OpenTelemetry
    builder.Services.AddOpenTelemetry()
        .ConfigureResource(resource => resource
            .AddService(
                serviceName: "{{project_name}}.Api",
                serviceVersion: typeof(Program).Assembly.GetName().Version?.ToString() ?? "1.0.0"))
        .WithTracing(tracing => tracing
            .AddAspNetCoreInstrumentation()
            .AddHttpClientInstrumentation()
            .AddEntityFrameworkCoreInstrumentation()
            .AddSource("MassTransit")
            .AddOtlpExporter())
        .WithMetrics(metrics => metrics
            .AddAspNetCoreInstrumentation()
            .AddHttpClientInstrumentation()
            .AddOtlpExporter()
            .AddPrometheusExporter());

    // Add services
    builder.Services.AddEndpointsApiExplorer();
    builder.Services.AddOpenApi();
    builder.Services.AddProblemDetails();

    // Add layers
    builder.Services.AddSharedServices();
    builder.Services.AddApplicationServices();
    builder.Services.AddInfrastructureServices(builder.Configuration);

    // Add FluentValidation
    builder.Services.AddValidatorsFromAssemblyContaining<Program>();

    // Configure MassTransit with RabbitMQ
    builder.Services.AddMassTransit(x =>
    {
        x.SetKebabCaseEndpointNameFormatter();
        
        // Add consumers from Application layer
        x.AddConsumers(typeof(ApplicationServiceCollectionExtensions).Assembly);

        x.UsingRabbitMq((context, cfg) =>
        {
            var connectionString = builder.Configuration.GetConnectionString("RabbitMq") 
                ?? "amqp://guest:guest@localhost:5672";
            
            cfg.Host(connectionString);
            cfg.ConfigureEndpoints(context);
        });
    });

    // Add Health Checks
    builder.Services.AddHealthChecks()
        .AddNpgSql(
            builder.Configuration.GetConnectionString("Database") ?? "",
            name: "database",
            tags: new[] { "db", "postgres" })
        .AddRabbitMQ(
            builder.Configuration.GetConnectionString("RabbitMq") ?? "amqp://guest:guest@localhost:5672",
            name: "rabbitmq",
            tags: new[] { "messaging", "rabbitmq" });

    // Configure CORS
    builder.Services.AddCors(options =>
    {
        options.AddDefaultPolicy(policy =>
        {
            policy.WithOrigins(builder.Configuration.GetSection("Cors:Origins").Get<string[]>() ?? new[] { "http://localhost:5000" })
                  .AllowAnyMethod()
                  .AllowAnyHeader();
        });
    });

    var app = builder.Build();

    // Configure the HTTP request pipeline
    app.UseExceptionHandler();
    
    if (app.Environment.IsDevelopment())
    {
        app.MapOpenApi();
        app.MapScalarApiReference(options =>
        {
            options.WithTitle("{{project_name}} API");
            options.WithTheme(ScalarTheme.BluePlanet);
        });
    }

    app.UseCors();
    app.UseSerilogRequestLogging();

    // Health checks
    app.MapHealthChecks("/health", new HealthCheckOptions
    {
        ResponseWriter = UIResponseWriter.WriteHealthCheckUIResponse
    });

    app.MapHealthChecks("/health/ready", new HealthCheckOptions
    {
        Predicate = check => check.Tags.Contains("db") || check.Tags.Contains("messaging"),
        ResponseWriter = UIResponseWriter.WriteHealthCheckUIResponse
    });

    app.MapHealthChecks("/health/live", new HealthCheckOptions
    {
        Predicate = _ => false
    });

    // Prometheus metrics endpoint
    app.MapPrometheusScrapingEndpoint();

    // Map API endpoints
    app.MapItemEndpoints();

    app.Run();
}
catch (Exception ex)
{
    Log.Fatal(ex, "Application terminated unexpectedly");
}
finally
{
    Log.CloseAndFlush();
}

// Required for WebApplicationFactory in tests
public partial class Program { }
