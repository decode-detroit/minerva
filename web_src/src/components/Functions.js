// Function to save the current configuration to a custom file
export async function saveConfig(filename) {
  // Save the configuration
  let saveConfig = {
    filename: filename,
  };
  fetch(`/saveConfig`, {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
    },
    body: JSON.stringify(saveConfig),
  }); // FIXME ignore errors
}

// Function to pass modifications to the system
export function saveEdits(modifications) {
  // Save the changes
  let editItem = {
    modifications: modifications,
  };
  fetch(`/edit`, {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
    },
    body: JSON.stringify(editItem),
  }); // Ignore errors
}

// Function to save the style change to the local stylesheet
export async function saveLocalStyle(newRule) {
  // Add the new rule to the local stylesheet
  let userStyles = document.getElementById("userStyles");
  userStyles.sheet.insertRule(newRule, userStyles.sheet.cssRules.length); // append to the end
}

// Function to save the style change
export async function saveStyles(newRules) {
  // Save the configuration
  let saveStyles = {
    newRules: newRules,
  };
  fetch(`/saveStyles`, {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
    },
    body: JSON.stringify(saveStyles),
  }); // FIXME ignore errors
}

// Helper function for asyncronous forEach
export async function asyncForEach(array, callback) {
  for (let index = 0; index < array.length; index++) {
    await callback(array[index], index, array);
  }
}

// Helper function to prevent clicks from continuing
export function stopPropogation(e) {
  // Prevent propogation
  e = e || window.event;
  e.stopPropagation();
}

// Helper functions for calculating box offset
export function vh(v) {
  var h = Math.max(document.documentElement.clientHeight, window.innerHeight || 0);
  return (v * h) / 100;
}

export function vw(v) {
  var w = Math.max(document.documentElement.clientWidth, window.innerWidth || 0);
  return (v * w) / 100;
}

export function vmin(v) {
  return Math.min(vh(v), vw(v));
}
