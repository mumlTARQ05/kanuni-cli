#!/usr/bin/env node

const fs = require('fs-extra');
const path = require('path');
const axios = require('axios');
const tar = require('tar');
const zlib = require('zlib');
const { pipeline } = require('stream/promises');
const chalk = require('chalk');
const ora = require('ora');
const crypto = require('crypto');

const REPO = 'v-lawyer/kanuni-cli';
const BINARY_NAME = 'kanuni';

/**
 * Get the platform-specific binary name
 */
function getBinaryName() {
  const platform = process.platform;
  const arch = process.arch;

  const platformMap = {
    'darwin-x64': 'kanuni-darwin-x64',
    'darwin-arm64': 'kanuni-darwin-arm64',
    'linux-x64': 'kanuni-linux-x64',
    'linux-arm64': 'kanuni-linux-arm64',
    'win32-x64': 'kanuni-windows-x64.exe'
  };

  const key = `${platform}-${arch}`;
  const binaryName = platformMap[key];

  if (!binaryName) {
    throw new Error(`Unsupported platform: ${platform} ${arch}`);
  }

  return binaryName;
}

/**
 * Get the latest release version from GitHub
 */
async function getLatestVersion() {
  const response = await axios.get(
    `https://api.github.com/repos/${REPO}/releases/latest`,
    {
      headers: {
        'User-Agent': 'kanuni-cli-npm'
      }
    }
  );
  return response.data.tag_name;
}

/**
 * Download and extract the binary
 */
async function downloadBinary(version, binaryName) {
  const isWindows = process.platform === 'win32';
  const extension = isWindows ? '.zip' : '.tar.gz';
  const downloadUrl = `https://github.com/${REPO}/releases/download/${version}/${binaryName}${extension}`;
  const checksumUrl = `${downloadUrl}.sha256`;

  const spinner = ora(`Downloading Kanuni CLI ${version}`).start();

  try {
    // Download checksum
    spinner.text = 'Downloading checksum...';
    const checksumResponse = await axios.get(checksumUrl, {
      responseType: 'text'
    });
    const expectedChecksum = checksumResponse.data.trim().split(' ')[0];

    // Download binary archive
    spinner.text = `Downloading ${binaryName}...`;
    const response = await axios.get(downloadUrl, {
      responseType: 'arraybuffer',
      onDownloadProgress: (progressEvent) => {
        const percentCompleted = Math.round((progressEvent.loaded * 100) / progressEvent.total);
        spinner.text = `Downloading ${binaryName}... ${percentCompleted}%`;
      }
    });

    // Verify checksum
    spinner.text = 'Verifying checksum...';
    const hash = crypto.createHash('sha256');
    hash.update(Buffer.from(response.data));
    const actualChecksum = hash.digest('hex');

    if (actualChecksum !== expectedChecksum) {
      throw new Error('Checksum verification failed');
    }

    // Extract binary
    spinner.text = 'Extracting binary...';
    const binDir = path.join(__dirname, 'bin');
    await fs.ensureDir(binDir);

    if (isWindows) {
      // For Windows, use a different extraction method
      // This is simplified - you'd need a proper zip extraction library
      const AdmZip = require('adm-zip');
      const zip = new AdmZip(Buffer.from(response.data));
      zip.extractAllTo(binDir, true);
    } else {
      // Save tar.gz to temp file and extract
      const tempFile = path.join(binDir, 'temp.tar.gz');
      await fs.writeFile(tempFile, response.data);

      await tar.extract({
        file: tempFile,
        cwd: binDir,
        strip: 0
      });

      await fs.remove(tempFile);
    }

    // Make binary executable (Unix only)
    if (!isWindows) {
      const binaryPath = path.join(binDir, isWindows ? binaryName : binaryName.replace('.tar.gz', ''));
      await fs.chmod(binaryPath, 0o755);
    }

    spinner.succeed(`Kanuni CLI ${version} installed successfully!`);
  } catch (error) {
    spinner.fail(`Failed to install Kanuni CLI: ${error.message}`);
    throw error;
  }
}

/**
 * Main installation function
 */
async function install() {
  console.log(chalk.cyan.bold('\n📦 Installing Kanuni CLI...\n'));

  try {
    const binaryName = getBinaryName();
    const version = await getLatestVersion();

    console.log(chalk.gray(`Platform: ${process.platform}`));
    console.log(chalk.gray(`Architecture: ${process.arch}`));
    console.log(chalk.gray(`Binary: ${binaryName}`));
    console.log(chalk.gray(`Version: ${version}\n`));

    await downloadBinary(version, binaryName);

    console.log(chalk.green.bold('\n✅ Installation complete!\n'));
    console.log(chalk.cyan('Run'), chalk.yellow('kanuni --help'), chalk.cyan('to get started\n'));
  } catch (error) {
    console.error(chalk.red.bold('\n❌ Installation failed!'));
    console.error(chalk.red(error.message));
    process.exit(1);
  }
}

// Run installation if this script is executed directly
if (require.main === module) {
  install();
}