# Build Stage
FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /src

# Copy solution and props files
COPY *.sln .
COPY Directory.Build.props .
COPY Directory.Packages.props .
COPY global.json .

# Copy project files
COPY src/{{project_name}}.Web/*.csproj src/{{project_name}}.Web/
COPY src/{{project_name}}.Api/*.csproj src/{{project_name}}.Api/
COPY src/{{project_name}}.Shared/*.csproj src/{{project_name}}.Shared/
COPY src/{{project_name}}.Domain/*.csproj src/{{project_name}}.Domain/
COPY src/{{project_name}}.Application/*.csproj src/{{project_name}}.Application/
COPY src/{{project_name}}.Infrastructure/*.csproj src/{{project_name}}.Infrastructure/

# Restore dependencies
RUN dotnet restore

# Copy everything else
COPY . .

# Build Web project
RUN dotnet publish src/{{project_name}}.Web -c Release -o /app/web

# Runtime Stage for Web
FROM mcr.microsoft.com/dotnet/aspnet:10.0 AS web
WORKDIR /app

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

COPY --from=build /app/web .
RUN chown -R appuser:appuser /app
USER appuser

ENV ASPNETCORE_URLS=http://+:5000
EXPOSE 5000

HEALTHCHECK --interval=30s --timeout=3s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:5000/health || exit 1

ENTRYPOINT ["dotnet", "{{project_name}}.Web.dll"]
