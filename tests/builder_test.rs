//! Builder pattern tests

use oif_aggregator::AggregatorBuilder;

#[tokio::test]
async fn test_aggregator_builder_default() {
    // Set required environment variable for tests
    std::env::set_var("INTEGRITY_SECRET", "test-secret-for-builder-tests-12345678901234567890");
    
    let result = AggregatorBuilder::default().start().await;
    assert!(result.is_ok());
    
    let (_app, _state) = result.unwrap();
    // Test that the builder creates a working aggregator
}

#[tokio::test]
async fn test_aggregator_builder_with_mock_adapter() {
    // Set required environment variable for tests
    std::env::set_var("INTEGRITY_SECRET", "test-secret-for-builder-tests-12345678901234567890");
    
    let mock_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
    let mock_solver = oif_aggregator::mocks::mock_solver();
    
    let result = AggregatorBuilder::default()
        .with_adapter(Box::new(mock_adapter))
        .expect("Failed to add adapter")
        .with_solver(mock_solver)
        .await
        .start()
        .await;
        
    assert!(result.is_ok());
    
    let (_app, state) = result.unwrap();
    // Verify we can get solver count
    let (_solvers, total_count, _, _) = state.solver_service.list_solvers_paginated(None, None).await.unwrap();
    assert_eq!(total_count, 1);
}

#[tokio::test]
async fn test_aggregator_builder_duplicate_adapter() {
    let mock_adapter1 = oif_aggregator::mocks::MockDemoAdapter::new();
    let mock_adapter2 = oif_aggregator::mocks::MockDemoAdapter::new(); // Same ID
    
    let result = AggregatorBuilder::default()
        .with_adapter(Box::new(mock_adapter1))
        .expect("Failed to add first adapter")
        .with_adapter(Box::new(mock_adapter2)); // Should fail
        
    assert!(result.is_err());
    // Just check that it fails - detailed error checking not needed for this test
}

#[tokio::test]
async fn test_aggregator_builder_multiple_adapters() {
    // Set required environment variable for tests
    std::env::set_var("INTEGRITY_SECRET", "test-secret-for-builder-tests-12345678901234567890");
    
    let demo_adapter = oif_aggregator::mocks::MockDemoAdapter::new();
    let test_adapter = oif_aggregator::mocks::MockTestAdapter::new();
    
    let demo_solver = oif_aggregator::mocks::mock_solver();
    let mut test_solver = oif_aggregator::mocks::mock_solver();
    test_solver.solver_id = "test-solver-different".to_string();
    test_solver.adapter_id = "mock-test-v1".to_string();
    
    let result = AggregatorBuilder::default()
        .with_adapter(Box::new(demo_adapter))
        .expect("Failed to add demo adapter")
        .with_adapter(Box::new(test_adapter))
        .expect("Failed to add test adapter")
        .with_solver(demo_solver)
        .await
        .with_solver(test_solver)
        .await
        .start()
        .await;
        
    assert!(result.is_ok());
    
    let (_app, state) = result.unwrap();
    let (_solvers, total_count, _, _) = state.solver_service.list_solvers_paginated(None, None).await.unwrap();
    assert_eq!(total_count, 2);
}