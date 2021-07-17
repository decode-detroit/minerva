import React from 'react';
import logoWide from './logo_wide.png';
import { EditMenu } from './components/Menus.js';
import { ViewArea } from './components/EditArea.js';
import { saveEdits } from './components/functions';
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
    }

    // Bind the various functions
    this.markSaved = this.markSaved.bind(this);
    this.saveModifications = this.saveModifications.bind(this);
  }

  // Update the saved status
  markSaved() {
    this.setState({
      saved: true,
    });
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
  
  // Render the complete application
  render() {
    return (
      <div className="app">
        <div className="header">
          <img src={logoWide} className="logo" alt="logo" />
          <EditMenu saved={this.state.saved} markSaved={this.markSaved}/>
        </div>
        <ViewArea saveModifications={this.saveModifications}/>
      </div>
    )
  }
}

export default App;
