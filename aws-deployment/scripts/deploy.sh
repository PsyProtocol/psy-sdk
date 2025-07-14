#!/bin/bash

set -e

# Configuration
PROJECT_NAME=${PROJECT_NAME:-qed-protocol}
AWS_REGION=${AWS_REGION:-ap-northeast-1}
AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
ECR_REPOSITORY="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com/${PROJECT_NAME}-rollup"
KEY_PAIR_NAME="${PROJECT_NAME}-deploy-key"
STACK_NAME="${PROJECT_NAME}-infrastructure"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to show help
show_help() {
    cat << 'EOF'
🚀 QED Protocol AWS Deployment Script

USAGE:
    ./deploy.sh [COMMAND] [OPTIONS]

COMMANDS:
    all      Deploy everything (infrastructure + services)
    clean    Clean up all AWS resources (force mode - no confirmations)

OPTIONS:
    --dry-run    Show what would be done without making changes

EXAMPLES:
    # Deploy everything from scratch
    ./deploy.sh all

    # Clean up everything (force mode - deletes all resources immediately)
    ./deploy.sh clean

ENVIRONMENT VARIABLES:
    PROJECT_NAME            Project name (default: qed-protocol)
    AWS_REGION             AWS region (default: ap-northeast-1)
    WORKER_INSTANCE_TYPE   EC2 instance type (default: c6i.2xlarge)
    ENVIRONMENT            Environment name (default: production)

MONITORING:
    - CloudFormation Console: https://${AWS_REGION}.console.aws.amazon.com/cloudformation/
    - ECS Console: https://${AWS_REGION}.console.aws.amazon.com/ecs/

TROUBLESHOOTING:
    If deployment fails, try:
    1. ./deploy.sh clean    # Clean up resources
    2. ./deploy.sh all      # Redeploy everything
EOF
}

# Function to check if AWS CLI is configured
check_aws_cli() {
    if ! command -v aws &> /dev/null; then
        log_error "❌ AWS CLI not found. Please install and configure AWS CLI first."
        exit 1
    fi
    
    if ! aws sts get-caller-identity &> /dev/null; then
        log_error "❌ AWS CLI not configured. Please run 'aws configure' first."
        exit 1
    fi
    
    log_info "✅ AWS CLI configured"
}

# Function to cleanup EBS volumes
cleanup_ebs_volumes() {
    log_info "🔍 Checking for orphaned EBS volumes..."
    
    # Get EBS volumes with QED-related tags (include both available and in-use)
    EBS_VOLUMES=$(aws ec2 describe-volumes --region "$AWS_REGION" \
        --filters "Name=tag:Project,Values=${PROJECT_NAME}" \
        --query 'Volumes[].VolumeId' \
        --output text 2>/dev/null || echo "")
    
    if [ -n "$EBS_VOLUMES" ]; then
        log_info "📋 Found EBS volumes: $EBS_VOLUMES"
        
        # Force detach and delete volumes
        for VOLUME_ID in $EBS_VOLUMES; do
            log_info "🔓 Force detaching EBS volume: $VOLUME_ID"
            # First try to detach if attached
            aws ec2 detach-volume --volume-id "$VOLUME_ID" --force --region "$AWS_REGION" 2>/dev/null || true
            
            # Wait for volume to become available
            log_info "⏳ Waiting for volume to be detached..."
            aws ec2 wait volume-available --volume-ids "$VOLUME_ID" --region "$AWS_REGION" 2>/dev/null || sleep 10
            
            log_info "🗑️  Deleting EBS volume: $VOLUME_ID"
            aws ec2 delete-volume --volume-id "$VOLUME_ID" --region "$AWS_REGION" || true
        done
    else
        log_info "✅ No orphaned EBS volumes found"
    fi
}

# Function to check if stack exists
check_stack_exists() {
    if aws cloudformation --no-cli-pager describe-stacks --stack-name "$STACK_NAME" --region "$AWS_REGION" &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Function to get stack status
get_stack_status() {
    aws cloudformation --no-cli-pager describe-stacks --stack-name "$STACK_NAME" --region "$AWS_REGION" \
        --query 'Stacks[0].StackStatus' --output text 2>/dev/null || echo "STACK_NOT_FOUND"
}

# Function to clean up orphaned EFS mount targets (from cleanup.sh)
cleanup_efs_mount_targets() {
    log_info "🔍 Checking for orphaned EFS mount targets..."
    
    # Get all mount targets in the region with availability zone info
    MOUNT_TARGETS=$(aws efs describe-mount-targets --region "$AWS_REGION" --query 'MountTargets[?contains(FileSystemId, `fs-`)].[MountTargetId,FileSystemId,SubnetId,AvailabilityZoneName]' --output text 2>/dev/null || echo "")
    
    if [ -n "$MOUNT_TARGETS" ]; then
        log_info "📋 Found EFS mount targets:"
        echo "Mount Target ID | File System ID | Subnet ID | Availability Zone"
        echo "----------------|----------------|-----------|------------------"
        echo "$MOUNT_TARGETS" | while read -r mt_id fs_id subnet_id az_name; do
            [ -n "$mt_id" ] && echo "$mt_id | $fs_id | $subnet_id | $az_name"
        done
        echo ""
        
        # Clean command is always force mode - no prompts
        if true; then
            echo "$MOUNT_TARGETS" | while read -r MOUNT_TARGET_ID FILE_SYSTEM_ID SUBNET_ID AZ_NAME; do
                if [ -n "$MOUNT_TARGET_ID" ]; then
                    log_info "🗑️  Deleting mount target: $MOUNT_TARGET_ID (AZ: $AZ_NAME)"
                    aws efs delete-mount-target --mount-target-id "$MOUNT_TARGET_ID" --region "$AWS_REGION" || true
                fi
            done
            
            log_info "⏳ Waiting for mount targets to be deleted..."
            sleep 30
            
            # Verify deletion
            log_info "🔍 Verifying mount target deletion..."
            REMAINING_TARGETS=$(aws efs describe-mount-targets --region "$AWS_REGION" --query 'length(MountTargets)' --output text 2>/dev/null || echo "0")
            if [ "$REMAINING_TARGETS" = "0" ]; then
                log_info "✅ All mount targets deleted successfully"
            else
                log_warning "⚠️  $REMAINING_TARGETS mount targets still exist"
            fi
        fi
    else
        log_info "✅ No orphaned EFS mount targets found"
    fi
}

# Function to clean up orphaned EFS file systems (from cleanup.sh)
cleanup_efs_file_systems() {
    log_info "🔍 Checking for orphaned EFS file systems..."
    
    # Get EFS file systems with QED-related tags or names
    EFS_SYSTEMS=$(aws efs describe-file-systems --region "$AWS_REGION" \
        --query 'FileSystems[?contains(Name, `qed`) || contains(Name, `QED`)].FileSystemId' \
        --output text 2>/dev/null || echo "")
    
    if [ -n "$EFS_SYSTEMS" ]; then
        log_info "📋 Found EFS file systems: $EFS_SYSTEMS"
        
        # Clean command is always force mode - no prompts
        if true; then
            for FS_ID in $EFS_SYSTEMS; do
                log_info "🗑️  Deleting EFS file system: $FS_ID"
                aws efs delete-file-system --file-system-id "$FS_ID" --region "$AWS_REGION" || true
            done
        fi
    else
        log_info "✅ No orphaned EFS file systems found"
    fi
}

# Function to clean up ECR images and repository
cleanup_ecr_images() {
    log_info "🔍 Checking for ECR repository and images..."
    
    # Check if ECR repository exists
    if aws ecr --no-cli-pager describe-repositories --repository-names ${PROJECT_NAME}-rollup --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "📋 Found ECR repository: ${PROJECT_NAME}-rollup"
        
        # Get list of images
        IMAGES=$(aws ecr --no-cli-pager list-images --repository-name ${PROJECT_NAME}-rollup --region "$AWS_REGION" --query 'imageIds[].imageTag' --output text 2>/dev/null || echo "")
        
        if [ -n "$IMAGES" ]; then
            log_info "📋 Found Docker images: $IMAGES"
        fi
        
        # Clean command is always force mode - no prompts
        if true; then
            log_info "🗑️  Deleting ECR repository: ${PROJECT_NAME}-rollup (with all images)"
            aws ecr --no-cli-pager delete-repository --repository-name ${PROJECT_NAME}-rollup --force --region "$AWS_REGION" || true
            log_info "✅ ECR repository deleted successfully"
        fi
    else
        log_info "✅ No ECR repository found"
    fi
}

# Function to force cleanup ECS services before stack deletion
force_cleanup_ecs() {
    log_info "🔧 Force cleaning ECS services..."
    
    # Stop all ECS services
    local cluster_name="${PROJECT_NAME}-cluster"
    local services=$(aws ecs --no-cli-pager list-services --cluster "$cluster_name" --region "$AWS_REGION" --query "serviceArns[]" --output text 2>/dev/null || echo "")
    
    if [ -n "$services" ]; then
        log_info "🛑 Stopping ECS services..."
        for service in $services; do
            local service_name=$(basename "$service")
            log_info "   Stopping service: $service_name"
            aws ecs --no-cli-pager update-service --cluster "$cluster_name" --service "$service_name" --desired-count 0 --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
        
        # Wait for services to stop
        sleep 30
        
        # Delete services
        log_info "🗑️  Deleting ECS services..."
        for service in $services; do
            local service_name=$(basename "$service")
            log_info "   Deleting service: $service_name"
            aws ecs --no-cli-pager delete-service --cluster "$cluster_name" --service "$service_name" --force --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
    fi
    
    # Deregister container instances
    local instances=$(aws ecs --no-cli-pager list-container-instances --cluster "$cluster_name" --region "$AWS_REGION" --query "containerInstanceArns[]" --output text 2>/dev/null || echo "")
    if [ -n "$instances" ]; then
        log_info "🗑️  Deregistering container instances..."
        for instance in $instances; do
            aws ecs --no-cli-pager deregister-container-instance --cluster "$cluster_name" --container-instance "$instance" --force --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
    fi
    
    # Delete cluster
    log_info "🗑️  Deleting ECS cluster..."
    aws ecs --no-cli-pager delete-cluster --cluster "$cluster_name" --region "$AWS_REGION" >/dev/null 2>&1 || true
}

# Function to force cleanup Auto Scaling Groups
force_cleanup_asg() {
    log_info "🔧 Force cleaning Auto Scaling Groups..."
    
    local asgs=$(aws autoscaling --no-cli-pager describe-auto-scaling-groups --region "$AWS_REGION" --query "AutoScalingGroups[?contains(AutoScalingGroupName, '${PROJECT_NAME}')].AutoScalingGroupName" --output text 2>/dev/null || echo "")
    
    if [ -n "$asgs" ]; then
        for asg in $asgs; do
            log_info "🗑️  Deleting Auto Scaling Group: $asg"
            # First, set desired capacity to 0
            aws autoscaling --no-cli-pager update-auto-scaling-group --auto-scaling-group-name "$asg" --desired-capacity 0 --min-size 0 --region "$AWS_REGION" >/dev/null 2>&1 || true
            sleep 10
            # Force delete the ASG
            aws autoscaling --no-cli-pager delete-auto-scaling-group --auto-scaling-group-name "$asg" --force-delete --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
        
        # Wait for instances to terminate
        log_info "⏳ Waiting for instances to terminate..."
        sleep 60
    fi
}

# Function to delete CloudFormation stack with retries
delete_stack() {
    local status=$(get_stack_status)
    
    case $status in
        "STACK_NOT_FOUND")
            log_info "✅ Stack does not exist"
            return 0
            ;;
        "DELETE_FAILED")
            log_warning "⚠️  Stack in DELETE_FAILED state. Forcing cleanup..."
            # Force cleanup problematic resources first
            force_cleanup_ecs
            force_cleanup_asg
            sleep 30
            
            # Retry stack deletion
            log_info "🔄 Retrying stack deletion..."
            aws cloudformation --no-cli-pager delete-stack --stack-name "$STACK_NAME" --region "$AWS_REGION"
            ;;
        "DELETE_IN_PROGRESS")
            log_info "⏳ Stack deletion already in progress. Waiting..."
            ;;
        "ROLLBACK_IN_PROGRESS")
            log_info "🔄 Stack rollback in progress. Waiting for completion..."
            ;;
        "CREATE_IN_PROGRESS")
            log_warning "⚠️  Stack stuck in CREATE_IN_PROGRESS state. Force deleting..."
            # Force cleanup problematic resources first
            force_cleanup_ecs
            force_cleanup_asg
            sleep 30
            
            # Force delete the stuck stack
            log_info "🔄 Force deleting stuck stack..."
            aws cloudformation --no-cli-pager delete-stack --stack-name "$STACK_NAME" --region "$AWS_REGION"
            ;;
        *)
            log_info "🗑️  Deleting CloudFormation stack: $STACK_NAME"
            aws cloudformation --no-cli-pager delete-stack --stack-name "$STACK_NAME" --region "$AWS_REGION"
            ;;
    esac
    
    log_info "⏳ Waiting for stack deletion to complete..."
    
    # Wait with timeout and retry mechanism (max 5 minutes per attempt)
    local max_attempts=3
    local attempt=1
    local timeout_seconds=300  # 5 minutes
    
    while [ $attempt -le $max_attempts ]; do
        log_info "   Attempt $attempt/$max_attempts (timeout: ${timeout_seconds}s)"
        
        # Use timeout command to limit wait time
        if timeout $timeout_seconds aws cloudformation --no-cli-pager wait stack-delete-complete --stack-name "$STACK_NAME" --region "$AWS_REGION" 2>/dev/null; then
            log_info "✅ Stack deleted successfully"
            return 0
        else
            local current_status=$(get_stack_status)
            log_info "Current stack status: $current_status"
            
            if [ "$current_status" = "STACK_NOT_FOUND" ]; then
                log_info "✅ Stack deleted successfully"
                return 0
            elif [ "$current_status" = "DELETE_FAILED" ] && [ $attempt -lt $max_attempts ]; then
                log_warning "⚠️  Stack deletion failed. Retrying with force cleanup..."
                force_cleanup_ecs
                force_cleanup_asg
                sleep 30
                aws cloudformation --no-cli-pager delete-stack --stack-name "$STACK_NAME" --region "$AWS_REGION"
                ((attempt++))
            elif [ "$current_status" = "DELETE_IN_PROGRESS" ]; then
                log_warning "⚠️  Stack deletion still in progress after timeout. Continuing anyway..."
                log_info "Stack will be cleaned up in background. EBS cleanup will proceed."
                return 0  # Continue with EBS cleanup
            else
                log_warning "⚠️  Stack deletion failed with status: $current_status"
                if [ $attempt -eq $max_attempts ]; then
                    log_warning "Final attempt failed. Continuing with resource cleanup..."
                    return 0  # Continue anyway for force cleanup
                fi
                break
            fi
        fi
    done
    
    log_warning "⚠️  Stack deletion attempts exhausted. Continuing with orphaned resource cleanup..."
    return 0  # Continue with EBS and other cleanup
}

# Function to verify cleanup
verify_cleanup() {
    log_info "🔍 Verifying cleanup..."
    
    # Check stack status
    local status=$(get_stack_status)
    if [ "$status" = "STACK_NOT_FOUND" ]; then
        log_info "✅ CloudFormation stack removed"
    else
        log_warning "⚠️  CloudFormation stack still exists with status: $status"
    fi
    
    # Check EFS resources
    local mount_targets=$(aws efs describe-mount-targets --region "$AWS_REGION" --query 'length(MountTargets)' --output text 2>/dev/null || echo "0")
    if [ "$mount_targets" = "0" ]; then
        log_info "✅ No EFS mount targets found"
    else
        log_warning "⚠️  $mount_targets EFS mount targets still exist"
    fi
    
    # Check ECR repository
    if aws ecr --no-cli-pager describe-repositories --repository-names ${PROJECT_NAME}-rollup --region "$AWS_REGION" >/dev/null 2>&1; then
        log_warning "⚠️  ECR repository still exists"
    else
        log_info "✅ ECR repository removed"
    fi
}

# Create SSH key pair
create_ssh_key() {
    log_info "Creating SSH key pair..."

    # Check if key pair already exists
    if aws ec2 describe-key-pairs --key-names ${KEY_PAIR_NAME} --region ${AWS_REGION} >/dev/null 2>&1; then
        log_info "Key pair ${KEY_PAIR_NAME} already exists."
        return
    fi

    # Generate ED25519 key pair in current directory
    ssh-keygen -t ed25519 -f ./${KEY_PAIR_NAME} -N "" -C "${PROJECT_NAME}-deployment-key"

    # Import public key to AWS
    aws ec2 import-key-pair \
        --key-name ${KEY_PAIR_NAME} \
        --public-key-material fileb://./${KEY_PAIR_NAME}.pub \
        --region ${AWS_REGION}

    log_info "SSH key pair created: ./${KEY_PAIR_NAME}"
    log_info "Connect to instances with: ssh -i ./${KEY_PAIR_NAME} ubuntu@<instance-ip>"
}

# Delete SSH key pair
delete_ssh_key() {
    log_info "Deleting SSH key pair..."

    # Delete from AWS
    aws ec2 delete-key-pair --key-name ${KEY_PAIR_NAME} --region ${AWS_REGION} || true

    # Delete local files
    rm -f ./${KEY_PAIR_NAME} ./${KEY_PAIR_NAME}.pub

    log_info "SSH key pair deleted."
}

# Monitor CloudFormation stack progress with detailed resource tracking
monitor_stack_progress() {
    local stack_name=$1
    local last_event_time=""
    local stack_status=""
    
    log_info "🚀 Starting CloudFormation deployment monitoring: $stack_name"
    log_info "📱 Real-time monitoring page: https://${AWS_REGION}.console.aws.amazon.com/cloudformation/home?region=${AWS_REGION}#/stacks/stackinfo?stackId=${stack_name}"
    echo ""
    
    while true; do
        # Get current stack status
        stack_status=$(aws cloudformation --no-cli-pager describe-stacks \
            --stack-name "$stack_name" \
            --region ${AWS_REGION} \
            --query "Stacks[0].StackStatus" \
            --output text 2>/dev/null || echo "STACK_NOT_EXISTS")
        
        # Check if stack is complete
        case "$stack_status" in
            "CREATE_COMPLETE"|"UPDATE_COMPLETE")
                echo ""
                log_info "🎉 Stack deployment completed successfully!"
                show_resource_summary "$stack_name"
                break
                ;;
            "CREATE_FAILED"|"UPDATE_FAILED"|"ROLLBACK_COMPLETE"|"UPDATE_ROLLBACK_COMPLETE")
                echo ""
                log_error "💥 Stack deployment failed: $stack_status"
                echo ""
                log_error "Failed event details:"
                aws cloudformation --no-cli-pager describe-stack-events \
                    --stack-name "$stack_name" \
                    --region ${AWS_REGION} \
                    --query "StackEvents[?ResourceStatus=='CREATE_FAILED' || ResourceStatus=='UPDATE_FAILED'].[Timestamp,LogicalResourceId,ResourceStatusReason]" \
                    --output text 2>/dev/null | while IFS=$'\t' read -r time resource reason; do
                    [ -n "$time" ] && echo "    $time - $resource: $reason"
                done || true
                return 1
                ;;
            "STACK_NOT_EXISTS")
                log_error "❌ Stack does not exist: $stack_name"
                return 1
                ;;
        esac
        
        # Show current progress with resource counts
        show_deployment_progress "$stack_name"
        
        # Get and display recent events
        local events=$(aws cloudformation --no-cli-pager describe-stack-events \
            --stack-name "$stack_name" \
            --region ${AWS_REGION} \
            --query "StackEvents[0:3].[Timestamp,LogicalResourceId,ResourceType,ResourceStatus,ResourceStatusReason]" \
            --output text 2>/dev/null)
        
        if [ -n "$events" ]; then
            echo "📋 Latest events:"
            echo "$events" | while IFS=$'\t' read -r timestamp resource_id resource_type status reason; do
                if [ -n "$timestamp" ]; then
                    local time_formatted=$(date -d "$timestamp" '+%H:%M:%S' 2>/dev/null || echo "$timestamp")
                    local short_resource=$(echo "$resource_id" | cut -c1-25)
                    case "$status" in
                        *"IN_PROGRESS")
                            echo "    🔄 $time_formatted │ $short_resource │ $status"
                            ;;
                        *"COMPLETE")
                            echo "    ✅ $time_formatted │ $short_resource │ $status"
                            ;;
                        *"FAILED")
                            echo "    ❌ $time_formatted │ $short_resource │ $status"
                            [ -n "$reason" ] && echo "       └─ Reason: $reason"
                            ;;
                        *)
                            echo "    ℹ️  $time_formatted │ $short_resource │ $status"
                            ;;
                    esac
                fi
            done
        fi
        
        echo ""
        sleep 10
    done
}

# Show deployment progress with resource counts and percentages
show_deployment_progress() {
    local stack_name=$1
    
    # Get resource counts
    local total_resources=$(aws cloudformation --no-cli-pager list-stack-resources \
        --stack-name "$stack_name" \
        --region ${AWS_REGION} \
        --query "length(StackResourceSummaries)" \
        --output text 2>/dev/null || echo "0")
    
    local completed_resources=$(aws cloudformation --no-cli-pager list-stack-resources \
        --stack-name "$stack_name" \
        --region ${AWS_REGION} \
        --query "length(StackResourceSummaries[?ResourceStatus=='CREATE_COMPLETE'])" \
        --output text 2>/dev/null || echo "0")
    
    local in_progress_resources=$(aws cloudformation --no-cli-pager list-stack-resources \
        --stack-name "$stack_name" \
        --region ${AWS_REGION} \
        --query "length(StackResourceSummaries[?contains(ResourceStatus, 'IN_PROGRESS')])" \
        --output text 2>/dev/null || echo "0")
    
    local failed_resources=$(aws cloudformation --no-cli-pager list-stack-resources \
        --stack-name "$stack_name" \
        --region ${AWS_REGION} \
        --query "length(StackResourceSummaries[?contains(ResourceStatus, 'FAILED')])" \
        --output text 2>/dev/null || echo "0")
    
    # Calculate percentage
    local percentage=0
    if [ "$total_resources" -gt 0 ]; then
        percentage=$((completed_resources * 100 / total_resources))
    fi
    
    # Get stack status
    local stack_status=$(aws cloudformation --no-cli-pager describe-stacks \
        --stack-name "$stack_name" \
        --region ${AWS_REGION} \
        --query "Stacks[0].StackStatus" \
        --output text 2>/dev/null || echo "UNKNOWN")
    
    # Show progress bar
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📊 Deployment progress: ${percentage}% (${completed_resources}/${total_resources} resources completed)"
    echo "🔄 Status: $stack_status"
    
    if [ "$in_progress_resources" -gt 0 ]; then
        echo "⏳ Creating: $in_progress_resources resources"
        
        # Show what's currently being created
        aws cloudformation --no-cli-pager list-stack-resources \
            --stack-name "$stack_name" \
            --region ${AWS_REGION} \
            --query "StackResourceSummaries[?contains(ResourceStatus, 'IN_PROGRESS')].{Resource:LogicalResourceId,Type:ResourceType,Status:ResourceStatus}" \
            --output text 2>/dev/null | while IFS=$'\t' read -r resource type status; do
            echo "    🔄 $resource ($type)"
        done
    fi
    
    if [ "$failed_resources" -gt 0 ]; then
        echo "❌ Failed: $failed_resources resources"
    fi
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Show final resource summary
show_resource_summary() {
    local stack_name=$1
    
    echo ""
    echo "📝 Deployment completed resource summary:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    # Group resources by type
    aws cloudformation --no-cli-pager list-stack-resources \
        --stack-name "$stack_name" \
        --region ${AWS_REGION} \
        --query "StackResourceSummaries[].ResourceType" \
        --output text 2>/dev/null | tr '	' '\n' | sort | uniq -c | sort -nr | while read count type; do
        echo "  $count × $type"
    done
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check AWS CLI
    if ! command -v aws &> /dev/null; then
        log_error "AWS CLI not found. Please install AWS CLI."
        exit 1
    fi

    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker not found. Please install Docker."
        exit 1
    fi

    # Check AWS credentials
    if ! aws sts get-caller-identity &> /dev/null; then
        log_error "AWS credentials not configured. Please configure AWS CLI."
        exit 1
    fi

    # Check for existing problematic resources
    log_info "Checking for conflicting resources..."
    
    # Check for stack in failed state
    STACK_STATUS=$(aws cloudformation --no-cli-pager describe-stacks --stack-name ${STACK_NAME} --region ${AWS_REGION} --query "Stacks[0].StackStatus" --output text 2>/dev/null || echo "STACK_NOT_EXISTS")
    
    case "$STACK_STATUS" in
        "DELETE_FAILED")
            log_warning "Stack is in DELETE_FAILED state, attempting to force delete..."
            
            # Clean up ECS cluster manually first
            aws ecs --no-cli-pager list-container-instances --cluster ${PROJECT_NAME}-cluster --region ${AWS_REGION} --query "containerInstanceArns[]" --output text 2>/dev/null | while read instance; do
                if [ ! -z "$instance" ]; then
                    log_info "Force deregistering container instance: $(basename "$instance")"
                    aws ecs --no-cli-pager deregister-container-instance --cluster ${PROJECT_NAME}-cluster --container-instance "$instance" --force --region ${AWS_REGION} >/dev/null 2>&1 || true
                fi
            done
            
            # Wait a bit and retry deletion
            sleep 10
            log_info "Retrying stack deletion..."
            aws cloudformation --no-cli-pager delete-stack --stack-name ${STACK_NAME} --region ${AWS_REGION} || true
            
            # Wait for deletion to complete
            log_info "Waiting for stack deletion to complete..."
            for i in {1..12}; do
                CURRENT_STATUS=$(aws cloudformation --no-cli-pager describe-stacks --stack-name ${STACK_NAME} --region ${AWS_REGION} --query "Stacks[0].StackStatus" --output text 2>/dev/null || echo "DELETED")
                if [ "$CURRENT_STATUS" = "DELETED" ] || [ "$CURRENT_STATUS" = "DELETE_COMPLETE" ]; then
                    log_info "Stack successfully deleted"
                    break
                elif [ "$CURRENT_STATUS" = "DELETE_FAILED" ]; then
                    log_error "Stack deletion failed again. Please run clean command first."
                    exit 1
                fi
                log_info "Waiting for deletion... ($i/12)"
                sleep 10
            done
            ;;
        "CREATE_FAILED"|"ROLLBACK_FAILED"|"UPDATE_ROLLBACK_FAILED")
            log_warning "Stack is in failed state: $STACK_STATUS"
            log_warning "Run './aws-deployment/scripts/deploy.sh clean' first to clean up resources"
            # Clean command is always force mode - continue without prompting
            log_warning "Continuing with force cleanup..."
            ;;
        "ROLLBACK_IN_PROGRESS"|"DELETE_IN_PROGRESS"|"CREATE_IN_PROGRESS"|"UPDATE_IN_PROGRESS")
            log_error "Stack operation in progress: $STACK_STATUS. Please wait for completion."
            exit 1
            ;;
    esac

    log_info "Prerequisites check passed."
}

# Build and push Docker image
build_and_push_image() {
    log_info "Building Rust binaries..."
    
    # Build Rust binaries first
    if [ ! -f "target/release/qed_rollup_cli" ]; then
        log_info "Compiling Rust binaries..."
        cargo build --release --bin qed_rollup_cli || {
            log_error "Rust compilation failed"
            exit 1
        }
    else
        log_info "Using existing Rust binaries"
    fi

    log_info "Building Docker image..."

    # Ensure ECR repository exists
    aws ecr --no-cli-pager describe-repositories --repository-names ${PROJECT_NAME}-rollup --region ${AWS_REGION} 2>/dev/null || \
    aws ecr --no-cli-pager create-repository --repository-name ${PROJECT_NAME}-rollup --region ${AWS_REGION}

    # Configure Docker for better network reliability
    log_info "Configuring Docker for ECR push..."

    # Set Docker registry mirror timeout and retry settings
    export DOCKER_BUILDKIT=1
    export BUILDKIT_PROGRESS=plain

    # Build the image with BuildKit for better performance
    log_info "Building image with Docker BuildKit..."
    DOCKER_BUILDKIT=1 docker build \
        -t ${PROJECT_NAME}-rollup:latest \
        -f aws-deployment/docker/Dockerfile \
        . || {
        log_error "Docker build failed"
        exit 1
    }

    # Tag for ECR
    docker tag ${PROJECT_NAME}-rollup:latest ${ECR_REPOSITORY}:latest
    docker tag ${PROJECT_NAME}-rollup:latest ${ECR_REPOSITORY}:$(git rev-parse --short HEAD)

    # Test ECR connectivity
    log_info "Testing ECR connectivity..."
    ECR_ENDPOINT="${AWS_ACCOUNT_ID}.dkr.ecr.${AWS_REGION}.amazonaws.com"
    if ! curl -s --connect-timeout 10 "https://${ECR_ENDPOINT}" > /dev/null; then
        log_warning "ECR endpoint connectivity test failed, but continuing..."
    fi

    # Login to ECR with retry
    log_info "Logging in to ECR..."
    for i in {1..3}; do
        if aws ecr --no-cli-pager get-login-password --region ${AWS_REGION} | docker login --username AWS --password-stdin ${ECR_REPOSITORY}; then
            log_info "ECR login successful"
            break
        else
            log_warning "ECR login attempt $i failed, retrying in 10 seconds..."
            sleep 10
        fi
    done

    # Push to ECR with retry mechanism
    log_info "Pushing Docker image to ECR..."

    # Function to push with retry
    push_with_retry() {
        local image=$1
        local max_retries=5
        local delay=30

        for i in $(seq 1 $max_retries); do
            log_info "Push attempt $i for $image..."
            if docker push "$image"; then
                log_info "Successfully pushed $image"
                return 0
            else
                log_warning "Push attempt $i failed for $image"
                if [ $i -lt $max_retries ]; then
                    log_info "Waiting ${delay} seconds before retry..."
                    sleep $delay
                    # Exponential backoff
                    delay=$((delay * 2))

                    # Re-login to ECR before retry
                    log_info "Re-authenticating with ECR..."
                    aws ecr --no-cli-pager get-login-password --region ${AWS_REGION} | docker login --username AWS --password-stdin ${ECR_REPOSITORY} || true
                else
                    log_error "Failed to push $image after $max_retries attempts"
                    return 1
                fi
            fi
        done
    }

    # Push both tags with retry
    push_with_retry "${ECR_REPOSITORY}:latest" || exit 1
    push_with_retry "${ECR_REPOSITORY}:$(git rev-parse --short HEAD)" || exit 1

    log_info "Docker image pushed successfully."
}

# Deploy CloudFormation stacks
deploy_infrastructure() {
    log_info "Deploying infrastructure stack..."

    # Create SSH key first
    create_ssh_key

    # Check stack status and handle DELETE_FAILED state
    STACK_STATUS=$(aws cloudformation --no-cli-pager describe-stacks --stack-name ${STACK_NAME} --region ${AWS_REGION} --query "Stacks[0].StackStatus" --output text 2>/dev/null || echo "STACK_NOT_EXISTS")
    
    if [ "$STACK_STATUS" = "DELETE_FAILED" ]; then
        log_warning "Stack is in DELETE_FAILED state, forcing deletion..."
        
        # Get failed resources to delete manually if needed
        log_warning "Failed resources:"
        aws cloudformation --no-cli-pager describe-stack-events \
            --stack-name ${STACK_NAME} \
            --region ${AWS_REGION} \
            --query "StackEvents[?ResourceStatus=='DELETE_FAILED'].[LogicalResourceId,ResourceStatusReason]" \
            --output text 2>/dev/null | while IFS=$'\t' read -r resource reason; do
            [ -n "$resource" ] && echo "    - $resource: $reason"
        done || true
        
        # Force delete the stack
        aws cloudformation --no-cli-pager delete-stack --stack-name ${STACK_NAME} --region ${AWS_REGION} || true
        
        # Wait for deletion to complete (or fail again) with timeout
        log_info "Waiting for stack deletion to complete (max 3 minutes)..."
        timeout 180 aws cloudformation --no-cli-pager wait stack-delete-complete --stack-name ${STACK_NAME} --region ${AWS_REGION} || true
        
        # If delete still fails, we'll continue anyway as deploy will recreate
        sleep 10
    elif [ "$STACK_STATUS" != "STACK_NOT_EXISTS" ] && [ "$STACK_STATUS" != "DELETE_COMPLETE" ]; then
        log_info "Stack exists with status: $STACK_STATUS"
    fi


    log_info "Starting CloudFormation deployment..."
    log_info "📋 Monitor progress at: https://${AWS_REGION}.console.aws.amazon.com/cloudformation/home?region=${AWS_REGION}#/stacks/stackinfo?stackId=${STACK_NAME}"
    log_info "💻 Or use CLI: aws cloudformation --no-cli-pager describe-stack-events --stack-name ${STACK_NAME} --region ${AWS_REGION}"
    
    # Create S3 bucket for CloudFormation templates
    ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
    S3_BUCKET="cf-templates-${ACCOUNT_ID}-${AWS_REGION}"
    
    if aws s3 ls "s3://${S3_BUCKET}" 2>/dev/null; then
        log_info "Using existing S3 bucket: ${S3_BUCKET}"
    else
        log_info "Creating S3 bucket for CloudFormation templates: ${S3_BUCKET}"
        if [ "$AWS_REGION" = "us-east-1" ]; then
            aws s3 mb "s3://${S3_BUCKET}"
        else
            aws s3 mb "s3://${S3_BUCKET}" --region ${AWS_REGION}
        fi
    fi
    
    # Start deployment in background and monitor immediately
    aws cloudformation --no-cli-pager deploy \
        --template-file aws-deployment/cloudformation/main.yaml \
        --stack-name ${STACK_NAME} \
        --s3-bucket ${S3_BUCKET} \
        --parameter-overrides \
            ProjectName=${PROJECT_NAME} \
            Environment=${ENVIRONMENT:-production} \
            WorkerInstanceType=${WORKER_INSTANCE_TYPE:-c6i.4xlarge} \
            KeyPairName=${KEY_PAIR_NAME} \
            ScyllaDBInstanceType=${SCYLLA_INSTANCE_TYPE:-r6i.large} \
            ScyllaDBInstanceCount=${SCYLLA_INSTANCE_COUNT:-3} \
            ScyllaDBDataVolumeSize=${SCYLLA_DATA_VOLUME_SIZE:-1000} \
            ScyllaDBCommitLogVolumeSize=${SCYLLA_COMMITLOG_VOLUME_SIZE:-200} \
        --capabilities CAPABILITY_NAMED_IAM \
        --region ${AWS_REGION} \
        --no-fail-on-empty-changeset &
    
    # Store the PID
    DEPLOY_PID=$!
    
    # Wait a bit for deployment to start
    sleep 5
    
    # Monitor progress while deployment runs
    monitor_stack_progress "${STACK_NAME}"
    
    # Wait for deploy command to finish
    wait $DEPLOY_PID

    log_info "Infrastructure stack deployed successfully."
}

deploy_ecs_services() {
    log_info "Deploying ECS services..."
    
    # Initial wait for instances to start
    log_info "Waiting for EC2 instances to initialize..."
    sleep 60  # Give instances time to boot and start UserData scripts
    
    # Wait for ScyllaDB SSM parameters to be available
    log_info "Waiting for ScyllaDB endpoints to be configured..."
    local max_attempts=30
    local attempt=0
    local all_params_ready=false
    
    while [ $attempt -lt $max_attempts ] && [ "$all_params_ready" = "false" ]; do
        all_params_ready=true
        
        # Check if all required SSM parameters exist
        # Check ScyllaDB endpoints
        for param in "coordinator-endpoint" "realm0-endpoint" "realm32-endpoint"; do
            if ! aws ssm get-parameter --name "/${PROJECT_NAME}/scylladb/${param}" --region ${AWS_REGION} &>/dev/null; then
                all_params_ready=false
                log_warning "Waiting for /${PROJECT_NAME}/scylladb/${param}..."
                break
            fi
        done
        
        # Check Redis endpoints
        for param in "coordinator/endpoint" "realm0/endpoint" "realm32/endpoint"; do
            if ! aws ssm get-parameter --name "/${PROJECT_NAME}/redis/${param}" --region ${AWS_REGION} &>/dev/null; then
                all_params_ready=false
                log_warning "Waiting for /${PROJECT_NAME}/redis/${param}..."
                break
            fi
        done
        
        if [ "$all_params_ready" = "false" ]; then
            sleep 10
            ((attempt++))
        fi
    done
    
    if [ "$all_params_ready" = "false" ]; then
        log_error "ScyllaDB endpoints not configured after $max_attempts attempts"
        return 1
    fi
    
    log_info "✅ All ScyllaDB endpoints are configured"
    
    # Display all configured endpoints
    log_info "📍 Configured endpoints:"
    log_info "ScyllaDB:"
    for param in "coordinator-endpoint" "realm0-endpoint" "realm32-endpoint"; do
        endpoint=$(aws ssm get-parameter --name "/${PROJECT_NAME}/scylladb/${param}" --region ${AWS_REGION} --query 'Parameter.Value' --output text 2>/dev/null || echo "Error reading parameter")
        log_info "  /${PROJECT_NAME}/scylladb/${param}: $endpoint"
    done
    
    log_info "Redis:"
    for param in "coordinator/endpoint" "realm0/endpoint" "realm32/endpoint"; do
        endpoint=$(aws ssm get-parameter --name "/${PROJECT_NAME}/redis/${param}" --region ${AWS_REGION} --query 'Parameter.Value' --output text 2>/dev/null || echo "Error reading parameter")
        log_info "  /${PROJECT_NAME}/redis/${param}: $endpoint"
    done
    
    aws cloudformation --no-cli-pager deploy \
        --template-file aws-deployment/cloudformation/ecs-services.yaml \
        --stack-name ${PROJECT_NAME}-ecs-services \
        --s3-bucket ${S3_BUCKET} \
        --parameter-overrides \
            ProjectName=${PROJECT_NAME} \
            Environment=${ENVIRONMENT:-production} \
            ContainerImage=${ECR_REPOSITORY}:latest \
        --capabilities CAPABILITY_IAM \
        --region ${AWS_REGION} \
        --no-fail-on-empty-changeset
    
    log_info "ECS services deployed successfully."
    
    # Update RPC config with ALB URL
    update_rpc_config
}

# Function to update rpc.config with deployed ALB URL
update_rpc_config() {
    log_info "Updating rpc.config with deployed endpoints..."
    
    # Get ALB DNS name
    ALB_DNS=$(aws cloudformation describe-stacks \
        --stack-name ${PROJECT_NAME}-infrastructure \
        --region ${AWS_REGION} \
        --query 'Stacks[0].Outputs[?OutputKey==`ALBDNSName`].OutputValue' \
        --output text)
    
    if [ -z "$ALB_DNS" ]; then
        log_warning "Could not retrieve ALB DNS name"
        return
    fi
    
    # Create new rpc.config
    cat > rpc.config << EOF
{
	"users_per_realm": 4194304,
	"realm_configs": [
		{
			"id": 0,
			"rpc_url": [
				"http://${ALB_DNS}:8546"
			]
		},
		{
			"id": 32,
			"rpc_url": [
				"http://${ALB_DNS}:8547"
			]
		}
	],
	"coordinator_configs": [
		{
			"id": 0,
			"rpc_url": [
				"http://${ALB_DNS}:8545"
			]
		}
	]
}
EOF
    
    log_info "✅ Updated rpc.config with ALB URL: ${ALB_DNS}"
    
    # Print helpful log commands
    print_log_commands
}

# Function to print log viewing commands
print_log_commands() {
    cat << EOF

📋 Useful commands for monitoring:

# ECS Service Logs (replace TASK_ID with actual task ID):
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "coordinator-processor/coordinator-processor/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "coordinator-worker/coordinator-worker/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "coordinator-edge/coordinator-edge/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "realm0-processor/realm0-processor/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "realm0-worker/realm0-worker/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "realm0-edge/realm0-edge/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "realm32-processor/realm32-processor/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "realm32-worker/realm32-worker/TASK_ID" --follow --region ${AWS_REGION}
aws logs tail /ecs/${PROJECT_NAME} --log-stream-names "realm32-edge/realm32-edge/TASK_ID" --follow --region ${AWS_REGION}

# List all log streams:
aws logs describe-log-streams --log-group-name /ecs/${PROJECT_NAME} --region ${AWS_REGION} --query 'logStreams[*].logStreamName' --output table

# Redis logs (on Redis instance):
aws ssm send-command --instance-ids REDIS_INSTANCE_ID --document-name AWS-RunShellScript --parameters 'commands=["tail -f /var/log/redis/*.log"]' --region ${AWS_REGION}

# ScyllaDB logs (on ScyllaDB instances):
aws ssm send-command --instance-ids SCYLLA_INSTANCE_ID --document-name AWS-RunShellScript --parameters 'commands=["docker logs -f scylladb"]' --region ${AWS_REGION}

# Get instance IDs:
aws ec2 describe-instances --filters "Name=tag:Project,Values=${PROJECT_NAME}" "Name=instance-state-name,Values=running" --region ${AWS_REGION} --query 'Reservations[].Instances[].[Tags[?Key==\`Name\`].Value|[0],InstanceId]' --output table

EOF
}

# Main cleanup function (enhanced with specialized EFS cleanup)
comprehensive_cleanup() {
    log_info "🧹 QED Protocol AWS Comprehensive Cleanup (Force Mode)"
    log_info "======================================="
    log_info "Stacks: ${PROJECT_NAME}-ecs-services, ${PROJECT_NAME}-infrastructure"
    log_info "Region: $AWS_REGION"
    log_info "⚠️  This will delete ALL resources without confirmation!"
    log_info "======================================="

    check_aws_cli
    
    # Delete ECS services stack first
    local ecs_stack="${PROJECT_NAME}-ecs-services"
    log_info "🗑️  Deleting ECS services stack: $ecs_stack"
    if aws cloudformation --no-cli-pager describe-stacks --stack-name "$ecs_stack" &>/dev/null; then
        aws cloudformation --no-cli-pager delete-stack --stack-name "$ecs_stack"
        log_info "⏳ Waiting for ECS services stack deletion..."
        aws cloudformation --no-cli-pager wait stack-delete-complete --stack-name "$ecs_stack" 2>/dev/null || true
        log_info "✅ ECS services stack deleted"
    else
        log_info "ℹ️  ECS services stack does not exist"
    fi
    
    # Delete infrastructure stack
    log_info "📊 Current infrastructure stack status: $(get_stack_status)"
    
    if check_stack_exists; then
        log_info "🔄 Infrastructure stack exists. Proceeding with deletion..."
        delete_stack
    else
        log_info "ℹ️  Infrastructure stack does not exist. Checking for orphaned resources..."
    fi
    
    # Clean up any orphaned EBS volumes (specialized cleanup)
    cleanup_ebs_volumes
    
    # Clean up ECR repository and images
    cleanup_ecr_images
    
    # Delete SSH key pair
    delete_ssh_key
    
    # Verify cleanup
    verify_cleanup
    
    echo ""
    log_info "🎉 Cleanup completed!"
    log_info "   You can now run the deployment script: ./aws-deployment/scripts/deploy.sh all"
}

# Parse command line arguments
COMMAND=""
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        all|clean)
            COMMAND=$1
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# If no command specified, show help
if [ -z "$COMMAND" ]; then
    show_help
    exit 0
fi

# Execute commands
case $COMMAND in
    clean)
        comprehensive_cleanup
        ;;
    all)
        check_prerequisites
        build_and_push_image
        deploy_infrastructure
        deploy_ecs_services
        
        echo ""
        log_info "🎉 Complete deployment finished!"
        log_info "📊 Monitor your deployment:"
        log_info "   CloudFormation: https://${AWS_REGION}.console.aws.amazon.com/cloudformation/"
        log_info "   ECS: https://${AWS_REGION}.console.aws.amazon.com/ecs/"
        ;;
    *)
        log_error "Unknown command: $COMMAND"
        show_help
        exit 1
        ;;
esac
