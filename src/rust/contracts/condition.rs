use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{
    CodeReferenceTaskContext, CodeStackSignal, ImplementationAction, TaskDefinition, TaskKind,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Condition {
    Predicate(String),
    All { all_of: Vec<Condition> },
    Any { any_of: Vec<Condition> },
    Not { not: Box<Condition> },
}

pub struct ConditionContext<'a> {
    pub task: &'a TaskDefinition,
    pub context: &'a CodeReferenceTaskContext,
    pub stack_frameworks: &'a BTreeSet<String>,
    pub focus_tags: &'a [String],
    pub signal: &'a CodeStackSignal,
    pub current_track: &'a str,
}

impl Condition {
    pub fn evaluate(&self, ctx: &ConditionContext) -> bool {
        match self {
            Condition::Predicate(p) => evaluate_predicate(p, ctx),
            Condition::All { all_of } => all_of.iter().all(|c| c.evaluate(ctx)),
            Condition::Any { any_of } => any_of.iter().any(|c| c.evaluate(ctx)),
            Condition::Not { not } => !not.evaluate(ctx),
        }
    }
}

fn evaluate_predicate(predicate: &str, ctx: &ConditionContext) -> bool {
    let Some((namespace, value)) = predicate.split_once(':') else {
        log::warn!("invalid predicate format (missing ':'): {predicate}");
        return false;
    };
    match namespace {
        "task_owns" => evaluate_task_owns(value, ctx),
        "task_action" => evaluate_task_action(value, ctx.task),
        "task_kind" => evaluate_task_kind(value, ctx.task),
        "context" => evaluate_context(value, ctx.context),
        "stack_fw" => ctx.stack_frameworks.contains(value),
        "focus" => ctx.focus_tags.iter().any(|tag| tag == value),
        "fw" => ctx.signal.frameworks.iter().any(|fw| fw == value),
        "lang" => ctx.signal.language.as_deref() == Some(value),
        _ => {
            log::warn!("unknown predicate namespace: {namespace}");
            false
        }
    }
}

fn evaluate_task_owns(value: &str, ctx: &ConditionContext) -> bool {
    use crate::code_quality::*;
    let task = ctx.task;
    match value {
        "test_implementation" => task_owns_test_implementation(task),
        "frontend_implementation" => task_owns_frontend_implementation(task),
        "frontend_surface" => task_owns_frontend_surface(task),
        "api_contract" => task_owns_api_contract(task),
        "persistence" => task_owns_persistence(task),
        "logging_infrastructure" => task_owns_logging_infrastructure(task, ctx.context),
        "sql_schema" => task_owns_sql_schema(task),
        "sql_query" => task_owns_sql_query(task),
        "sql_transaction" => task_owns_sql_transaction(task),
        "sql_performance" => task_owns_sql_performance(task),
        "sql_analytics" => task_owns_sql_analytics(task),
        "sql_tests" => task_owns_sql_tests(task),
        "nest_service_boundary" => task_owns_nest_service_boundary(task),
        "typescript_type_modeling" => task_owns_typescript_type_modeling(task),
        "typescript_configuration" => task_owns_typescript_configuration(task),
        "typescript_pattern" => task_owns_typescript_pattern(task),
        _ => {
            log::warn!("unknown task_owns value: {value}");
            false
        }
    }
}

fn evaluate_task_action(value: &str, task: &TaskDefinition) -> bool {
    let action = match value {
        "CreateOrUpdateEntity" => ImplementationAction::CreateOrUpdateEntity,
        "CreateOrUpdatePersistence" => ImplementationAction::CreateOrUpdatePersistence,
        "CreateOrUpdateInterface" => ImplementationAction::CreateOrUpdateInterface,
        "CreateOrUpdateUiFlow" => ImplementationAction::CreateOrUpdateUiFlow,
        "CreateOrUpdateFrontendNavigation" => {
            ImplementationAction::CreateOrUpdateFrontendNavigation
        }
        "ImplementReactiveClientFlow" => ImplementationAction::ImplementReactiveClientFlow,
        "ImplementSharedClientState" => ImplementationAction::ImplementSharedClientState,
        "OptimizeFrontendPerformance" => ImplementationAction::OptimizeFrontendPerformance,
        "ImplementServerRenderedComponent" => {
            ImplementationAction::ImplementServerRenderedComponent
        }
        "ImplementServerMutation" => ImplementationAction::ImplementServerMutation,
        "ImplementFrontendFrameworkVersionFeature" => {
            ImplementationAction::ImplementFrontendFrameworkVersionFeature
        }
        "ImplementMobilePlatformBehavior" => ImplementationAction::ImplementMobilePlatformBehavior,
        "ImplementClientStorage" => ImplementationAction::ImplementClientStorage,
        "ImplementLanguageVersionFeature" => ImplementationAction::ImplementLanguageVersionFeature,
        "ImplementGenericTypeAbstraction" => ImplementationAction::ImplementGenericTypeAbstraction,
        "ImplementDependencyAbstraction" => ImplementationAction::ImplementDependencyAbstraction,
        "RefactorModuleStructure" => ImplementationAction::RefactorModuleStructure,
        "OptimizeRuntimePerformance" => ImplementationAction::OptimizeRuntimePerformance,
        "CreateOrUpdateStateMachine" => ImplementationAction::CreateOrUpdateStateMachine,
        "CreateOrUpdateBusinessRule" => ImplementationAction::CreateOrUpdateBusinessRule,
        "AddReferenceField" => ImplementationAction::AddReferenceField,
        "ValidateReferenceFormat" => ImplementationAction::ValidateReferenceFormat,
        "UseFixtureOrMockData" => ImplementationAction::UseFixtureOrMockData,
        "WireReferenceInApiOrUi" => ImplementationAction::WireReferenceInApiOrUi,
        "CreateEntityCrud" => ImplementationAction::CreateEntityCrud,
        "CreateEntityRepository" => ImplementationAction::CreateEntityRepository,
        "CreateEntityAdminPage" => ImplementationAction::CreateEntityAdminPage,
        "CreateEntityMigration" => ImplementationAction::CreateEntityMigration,
        "CreateOrUpdatePersistenceQuery" => ImplementationAction::CreateOrUpdatePersistenceQuery,
        "ImplementPersistenceTransaction" => ImplementationAction::ImplementPersistenceTransaction,
        "OptimizePersistenceQuery" => ImplementationAction::OptimizePersistenceQuery,
        "ImplementAnalyticalQuery" => ImplementationAction::ImplementAnalyticalQuery,
        "ImplementEntityLifecycle" => ImplementationAction::ImplementEntityLifecycle,
        "AddOrUpdateTests" => ImplementationAction::AddOrUpdateTests,
        "AddOrUpdatePersistenceTests" => ImplementationAction::AddOrUpdatePersistenceTests,
        "AddOrUpdateConfig" => ImplementationAction::AddOrUpdateConfig,
        "ImplementAuthenticationOrAuthorization" => {
            ImplementationAction::ImplementAuthenticationOrAuthorization
        }
        "ImplementAsyncProcessing" => ImplementationAction::ImplementAsyncProcessing,
        "ImplementCachePolicy" => ImplementationAction::ImplementCachePolicy,
        "ImplementExternalServiceIntegration" => {
            ImplementationAction::ImplementExternalServiceIntegration
        }
        "ImplementResiliencePolicy" => ImplementationAction::ImplementResiliencePolicy,
        "ConfigureServiceRoutingOrDiscovery" => {
            ImplementationAction::ConfigureServiceRoutingOrDiscovery
        }
        "ImplementObservability" => ImplementationAction::ImplementObservability,
        "MigrateFrameworkImplementation" => ImplementationAction::MigrateFrameworkImplementation,
        "ImplementFrontendExperienceContract" => {
            ImplementationAction::ImplementFrontendExperienceContract
        }
        "ImplementRuntimeDeliveryContract" => {
            ImplementationAction::ImplementRuntimeDeliveryContract
        }
        "RefactorSupportingCode" => ImplementationAction::RefactorSupportingCode,
        _ => {
            log::warn!("unknown task_action value: {value}");
            return false;
        }
    };
    task.implementation_actions.iter().any(|a| *a == action)
}

fn evaluate_task_kind(value: &str, task: &TaskDefinition) -> bool {
    let kind = match value {
        "FeatureIncrement" => TaskKind::FeatureIncrement,
        "DataModelIncrement" => TaskKind::DataModelIncrement,
        "InterfaceIncrement" => TaskKind::InterfaceIncrement,
        "UiFlowIncrement" => TaskKind::UiFlowIncrement,
        "FrontendExperience" => TaskKind::FrontendExperience,
        "RuntimeDelivery" => TaskKind::RuntimeDelivery,
        "RuntimeDeliveryClosure" => TaskKind::RuntimeDeliveryClosure,
        "BrowserQualityClosure" => TaskKind::BrowserQualityClosure,
        "IntegrationIncrement" => TaskKind::IntegrationIncrement,
        "VerificationIncrement" => TaskKind::VerificationIncrement,
        "RefactorSupport" => TaskKind::RefactorSupport,
        "ConfigurationSupport" => TaskKind::ConfigurationSupport,
        _ => {
            log::warn!("unknown task_kind value: {value}");
            return false;
        }
    };
    task.task_kind == kind
}

fn evaluate_context(value: &str, context: &CodeReferenceTaskContext) -> bool {
    match value {
        "security" => context.security,
        "async_processing" => context.async_processing,
        "integration" => context.integration,
        "resilience" => context.resilience,
        "observability" => context.observability,
        "request_tracing" => context.request_tracing,
        "application_architecture" => context.application_architecture,
        _ => {
            log::warn!("unknown context value: {value}");
            false
        }
    }
}
