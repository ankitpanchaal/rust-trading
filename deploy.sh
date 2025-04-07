#!/bin/bash

# Dynamic branch name (default to 'staging' if not provided)
BRANCH_NAME=${1:-staging}

# Switch to the specified branch
echo "Switching to branch: $BRANCH_NAME"
git checkout $BRANCH_NAME

# Pull the latest changes
echo "Pulling latest changes from branch: $BRANCH_NAME"
git pull origin $BRANCH_NAME

# Rebuild the Docker image
echo "Building the Docker image..."
docker build -t rust-trading-app .

# Run the Docker container
echo "Running the Docker container..."
docker run -p 5000:5000 --env-file .env rust-trading-app
