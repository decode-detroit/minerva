import React from 'react';
import logoWide from './logo_wide.png';
import { EditMenu } from './components/Menus.js';
import { ViewArea } from './components/EditArea.js';
import { saveEdits, saveConfig, saveLocalStyle, saveStyles } from './components/Functions';
import './App.css';

// The top level class
export class App extends React.PureComponent {  
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      saved: true,
      configFile: "",
      newRules: {}, // any new style rules (not yet saved)
    }

    // Bind the various functions
    this.saveFile = this.saveFile.bind(this);
    this.saveModifications = this.saveModifications.bind(this);
    this.handleFileChange = this.handleFileChange.bind(this);
    this.collectStyles = this.collectStyles.bind(this);
  }

  // Save any modification to the configuration
  saveModifications(modifications) {
    // Save the modifications
    saveEdits(modifications);

    // Mark changes as unsaved
    this.setState({
      saved: false,
    });
  }

  // Save the configuration to the current filename
  saveFile() {
    // Save the configuration with the current filename
    saveConfig(this.state.configFile);

    // Save the style changes (automatically sets the filename)
    let newRules = "";
    for (let [selector, rule] of Object.entries(this.state.newRules)) {
      newRules += `${selector} ${rule}\n`;
    };
    saveStyles(newRules)

    // Update the save state and clear the rules
    this.setState({
      saved: true,
      newRules: [],
    });
  }

  // Function to handle new text in the input
  handleFileChange(e) {
    // Save the new value as the filename
    this.setState({
      configFile: e.target.value,
    });
  }

  // Function to collect the new style rules
  collectStyles(newSelector, newRule) {
    // Append the new rule to the local stylesheet
    saveLocalStyle(`${newSelector} ${newRule}`);

    // Append it to the current styles
    this.setState((prevState) => {
      // Add or update the rule
      let newRules = {...prevState.newRules};
      newRules[`${newSelector}`] = newRule;

      // Return the update
      return {
        newRules: newRules,
        saved: false,
      };
    });
  }

  // On render, pull the configuration file path
  async componentDidMount() {
    // Retrieve the configuation path
    const response = await fetch(`getConfigPath`);
    const json = await response.json();

    // If valid, save configuration
    if (json.path.isValid) {
      this.setState({
        configFile: json.path.path, // FIXME Allow for more subtlety
      });
    }
  }
  
  // Render the complete application
  render() {
    return (
      <div className="app">
        <div className="header">
          <img src={logoWide} className="logo" alt="logo" />
          <EditMenu saved={this.state.saved} filename={this.state.configFile} handleFileChange={this.handleFileChange} saveFile={this.saveFile} />
        </div>
        <ViewArea saveModifications={this.saveModifications} saveStyle={this.collectStyles} />
      </div>
    )
  }
}

export default App;
