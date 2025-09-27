#!/usr/bin/env node

const fs = require('fs-extra');
const path = require('path');
const chalk = require('chalk');

async function uninstall() {
  try {
    const binDir = path.join(__dirname, 'bin');

    if (await fs.pathExists(binDir)) {
      await fs.remove(binDir);
      console.log(chalk.green('✅ Kanuni CLI uninstalled successfully'));
    }
  } catch (error) {
    console.error(chalk.red(`Error during uninstall: ${error.message}`));
  }
}

if (require.main === module) {
  uninstall();
}