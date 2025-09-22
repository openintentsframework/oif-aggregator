//! Metrics collection and time-series types
//!
//! This module provides types for collecting and storing time-series metrics data
//! for solvers and aggregation operations. It's designed to support both real-time
//! circuit breaker decisions and historical analysis for scoring engines.

pub mod time_series;

pub use time_series::{
	ErrorType, MetricsAggregate, MetricsBucket, MetricsComputationError, MetricsDataPoint,
	MetricsTimeSeries, Operation, OperationStats, RollingMetrics, TimeBucket, TimeRange,
	TimeWindow,
};
