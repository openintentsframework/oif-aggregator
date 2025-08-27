# Maintenance and Cleanup Configuration

This document describes the maintenance and cleanup features of the OIF Aggregator.

## Order Cleanup

The OIF Aggregator automatically cleans up old orders in final status to prevent memory leaks and maintain system performance.

### Configuration

Orders cleanup can be configured through:


#### Configuration File
```json
{
  "maintenance": {
    "order_retention_days": 30
  }
}
```

**Note**: The `maintenance` section is completely optional in configuration files. If omitted, default values will be used.

#### 3. Default Value
If no configuration is provided, orders are retained for **10 days** by default.

### How It Works

- **Scheduled Job**: Runs automatically every 24 hours
- **Target Orders**: Only orders with final status (`Finalized` or `Failed`)
- **Age Criteria**: Orders older than the configured retention period
- **Storage Cleanup**: Removes orders from in-memory storage to free up memory

### Behavior

1. **System Startup**: Schedules daily cleanup job (doesn't run immediately)
2. **Daily Execution**: Job processor triggers cleanup every 24 hours
3. **Order Selection**: Finds orders in final status older than retention period
4. **Memory Cleanup**: Deletes old orders from storage
5. **Logging**: Reports count of deleted orders
