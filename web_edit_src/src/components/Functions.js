// Function to change the current port for the browser window
export async function switchPort(siteport) {
  window.location.port = siteport;
}

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

// Function to save the style change
export async function saveStyle(selector, rule) {
  // Add the new rule to the local stylesheet
  let userStyles = document.getElementById("userStyles");
  userStyles.sheet.insertRule(`${selector} ${rule}`, userStyles.sheet.cssRules.length); // append to the end

  // Save the configuration
  let newStyles = {};
  newStyles[`${selector}`] = `${rule}`;
  let saveStyles = {
    newStyles: newStyles,
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

// Helper function to extract the edit location from an itempair
// WARNING: Transitional - will soon be removed
export function getLocation(itemPair) {
  // Placeholder values
  let location = null;

  // Match the display type on the itemPair
  if (itemPair.display.DisplayControl && itemPair.display.DisplayControl.edit_location) {
    location = { left: itemPair.display.DisplayControl.edit_location[0], top: itemPair.display.DisplayControl.edit_location[1] };
  
  } else if (itemPair.display.DisplayWith && itemPair.display.DisplayWith.edit_location) {
    location = { left: itemPair.display.DisplayWith.edit_location[0], top: itemPair.display.DisplayWith.edit_location[1] };

  } else if (itemPair.display.DisplayDebug && itemPair.display.DisplayDebug.edit_location) {
    location = { left: itemPair.display.DisplayDebug.edit_location[0], top: itemPair.display.DisplayDebug.edit_location[1] };

  } else if (itemPair.display.LabelControl && itemPair.display.LabelControl.edit_location) {
    location = { left: itemPair.display.LabelControl.edit_location[0], top: itemPair.display.LabelControl.edit_location[1] };

  } else if (itemPair.display.LabelHidden && itemPair.display.LabelHidden.edit_location) {
    location = { left: itemPair.display.LabelHidden.edit_location[0], top: itemPair.display.LabelHidden.edit_location[1] };

  } else if (itemPair.display.Hidden && itemPair.display.Hidden.edit_location) {
    location = { left: itemPair.display.Hidden.edit_location[0], top: itemPair.display.Hidden.edit_location[1] };
  }

  // Return the location
  return location;
}

// Helper function to update the edit location in an itempair
// WARNING: Transitional - will soon be removed
export function clearLocation(itemPair) {
  // Match the display type on the itemPair
  if (itemPair.display.DisplayControl) {
    itemPair.display.DisplayControl.edit_location = null;
  
  } else if (itemPair.display.DisplayWith) {
    itemPair.display.DisplayWith.edit_location = null;

  } else if (itemPair.display.DisplayDebug) {
    itemPair.display.DisplayDebug.edit_location = null;

  } else if (itemPair.display.LabelControl) {
    itemPair.display.LabelControl.edit_location = null;

  } else if (itemPair.display.LabelHidden) {
    itemPair.display.LabelHidden.edit_location = null;

  } else if (itemPair.display.Hidden) {
    itemPair.display.Hidden.edit_location = null;
  }

  // Return the location
  return itemPair;
}
