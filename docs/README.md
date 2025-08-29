# OIF Aggregator Documentation

Welcome to the comprehensive documentation for the **Open Intent Framework (OIF) Aggregator**. This documentation provides everything you need to understand, configure, deploy, and integrate with the OIF Aggregator.

## 📚 Documentation Overview

### 🚀 **Getting Started**
- **[Quick Start Guide](quick-start.md)** - Step-by-step setup with working examples
- **[Configuration Guide](configuration.md)** - Complete configuration reference
- **[API Documentation](api/)** - Interactive API reference with Swagger UI

### 🏗️ **Understanding the System**
- **[Quotes & Aggregation Guide](quotes-and-aggregation.md)** - How quote aggregation works

### 🔧 **Operations & Deployment**
- **[Docker Guide](docker.md)** - Container deployment and development workflows
- **[Security Guide](security.md)** - Security best practices and considerations
- **[Maintenance Guide](maintenance.md)** - System maintenance and monitoring

### 🚀 **Development & Extension**
- **[Custom Adapter Guide](custom-adapters.md)** - How to implement and register custom solver adapters

### 🔗 **External Resources**
- **[Interactive API Docs](https://openintentsframework.github.io/oif-aggregator/)** - Live Swagger UI
- **[GitHub Repository](https://github.com/openintentsframework/oif-aggregator)** - Source code and issues

## 🔄 **Documentation Structure**

```
docs/
├── README.md                      # This overview page
├── quick-start.md                 # Step-by-step setup guide
├── configuration.md               # Complete configuration reference
├── quotes-and-aggregation.md     # Quote system and aggregation logic
├── custom-adapters.md             # Custom adapter implementation guide
├── docker.md                     # Docker deployment and development
├── security.md                   # Security guidelines and best practices
├── maintenance.md                 # System maintenance and monitoring
└── api/                           # API documentation
    ├── README.md                 # API documentation guide
    ├── index.html                # Interactive Swagger UI
    └── openapi.json              # OpenAPI 3.1 specification
```

## 📖 **What Each Document Covers**

### **[Quick Start Guide](quick-start.md)**
- Step-by-step setup from zero to running server
- Basic configuration examples that work
- API testing with real requests
- Programmatic usage examples
- Troubleshooting common setup issues

### **[Configuration Guide](configuration.md)**
- Environment variables and their usage
- JSON configuration file structure
- Server, security, and solver settings
- Timeout and rate limiting configuration
- Storage backend configuration

### **[Quotes & Aggregation Guide](quotes-and-aggregation.md)**
- Complete quotes endpoint documentation
- How aggregation logic works step-by-step
- Solver selection and ranking algorithms
- Integration examples and best practices
- Error handling and monitoring

### **[API Documentation](api/)**
- Interactive Swagger UI for testing endpoints
- Complete OpenAPI 3.1 specification
- Request/response schemas and examples
- Authentication and error handling

### **[Security Guide](security.md)**
- HMAC integrity verification configuration
- Integrity secret setup and best practices
- Secure secret generation methods
- Production security checklist

### **[Custom Adapter Guide](custom-adapters.md)**
- SolverAdapter trait implementation
- Registration and configuration
- Error handling and testing
- Best practices for custom integrations

### **[Docker Guide](docker.md)**
- Container deployment and development workflows
- Environment variable configuration
- Production and development Docker setups

### **[Maintenance Guide](maintenance.md)**
- Health monitoring and metrics
- Log management and debugging
- Performance tuning
- Troubleshooting common issues

## 🤝 **Contributing to Documentation**

Documentation improvements are always welcome! Here's how to contribute:

1. **API Documentation** - Auto-generated from code annotations using `utoipa`
2. **Configuration** - Update when adding new configuration options
3. **Security** - Add new security considerations or best practices
4. **Maintenance** - Include new operational procedures or troubleshooting

### Generating API Documentation
```bash
# Regenerate OpenAPI specification
./scripts/generate-openapi.sh

# The API docs are automatically deployed to GitHub Pages
# when changes are pushed to the main branch
```

## 🔍 **Need Help?**

- **Issues**: [GitHub Issues](https://github.com/openintentsframework/oif-aggregator/issues)
- **API Testing**: [Interactive Swagger UI](https://openintentsframework.github.io/oif-aggregator/)

---

**📝 Last Updated**: Documentation is kept up-to-date with each release. API documentation is automatically regenerated from code annotations.
