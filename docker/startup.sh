#!/bin/bash

echo "before"
ls -la

cd /app/proto

echo "after"
ls -la

# Check if requirements.txt exists and install dependencies
if [ -f "requirements.txt" ] || [ -L "requirements.txt" ]; then
    echo "Installing dependencies..."
    pip install --no-cache-dir -r requirements.txt
else
    echo "No requirements"
    ls -la
fi

cd /app

ls -la

# Run the application
echo "Starting application..."
python proto/app.py