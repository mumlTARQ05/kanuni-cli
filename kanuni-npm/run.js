#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * Get the binary path based on platform
 */
function getBinaryPath() {
  const platform = process.platform;
  const isWindows = platform === 'win32';
  const binaryName = isWindows ? 'kanuni.exe' : 'kanuni';
  const binDir = path.join(__dirname, 'bin');

  // Look for the binary in the bin directory
  const files = fs.readdirSync(binDir);
  const binary = files.find(file => file.includes('kanuni'));

  if (!binary) {
    throw new Error('Kanuni binary not found. Please run: npm install -g kanuni-cli');
  }

  return path.join(binDir, binary);
}

/**
 * Run the binary with passed arguments
 */
function run() {
  try {
    const binaryPath = getBinaryPath();

    // Check if binary exists
    if (!fs.existsSync(binaryPath)) {
      console.error('Error: Kanuni binary not found.');
      console.error('Please reinstall: npm install -g kanuni-cli');
      process.exit(1);
    }

    // Spawn the binary with all arguments passed to this script
    const child = spawn(binaryPath, process.argv.slice(2), {
      stdio: 'inherit',
      env: process.env,
      cwd: process.cwd()
    });

    child.on('error', (error) => {
      console.error(`Error: ${error.message}`);
      process.exit(1);
    });

    child.on('exit', (code) => {
      process.exit(code || 0);
    });
  } catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

// Run the binary
run();