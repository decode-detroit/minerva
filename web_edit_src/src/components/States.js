import React from 'react';
import { stopPropogation } from './Functions';
import { SendNode } from './Nodes';
import { SelectMenu } from './Menus';

// A State list element
export class State extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      description: "Loading ...",
    }

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.state.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        this.setState({
          description: json.data.item.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, update descriptions
  componentDidMount() {
    this.updateItem();
  }

  // On change of item id, update descriptions
  componentDidUpdate(prevProps, prevState) {
    // Update the item description, if it changed
    if (this.props.state.id !== prevProps.state.id) {
      this.updateItem();
    }
  }

  // Render the completed action
  render() {
    return (
      <>
        <div className="state">
          <div className="deleteState" onClick={(e) => {stopPropogation(e); this.props.removeState()}}>X</div>
          {this.state.description}
          <SendNode type="event" onPointerDown={(e) => {stopPropogation(e); this.props.grabFocus(this.props.state.id)}}/>
        </div>
      </>
    );
  }
}

// An Unmodifiable State list element
export class UnmodifiableState extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      description: "Loading ...",
    }

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.state.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        this.setState({
          description: json.data.item.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, update descriptions
  componentDidMount() {
    this.updateItem();
  }

  // On change of item id, update descriptions
  componentDidUpdate(prevProps, prevState) {
    // Update the item description, if it changed
    if (this.props.state.id !== prevProps.state.id) {
      this.updateItem();
    }
  }

  // Render the completed action
  render() {
    return (
      <>
        <div className="unmodifiableState">
          {this.state.description}
          <SendNode type="event" onPointerDown={(e) => {stopPropogation(e); this.props.grabFocus(this.props.state.id)}}/>
        </div>
      </>
    );
  }
}

// A State list element
export class SelectedEvent extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      description: "Loading ...",
      isMenuVisible: false,
    }

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.toggleMenu = this.toggleMenu.bind(this);
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.event.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        this.setState({
          description: json.data.item.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to show or hide the select menu
  toggleMenu() {
    // Pass the select menu upstream, if visible
    if (!this.state.isMenuVisible) {
      // Try to claim the select menu, return on failure
      if (!this.props.selectMenu(<SelectMenu type="event" closeMenu={this.toggleMenu} addItem={(id) => {this.toggleMenu(); this.props.changeEvent(id);}} />)) {
        return;
      }
    } else {
      this.props.selectMenu(null);
    }

    // Set the new state of the menu
    this.setState(prevState => {
      return ({
        isMenuVisible: !prevState.isMenuVisible,
      });
    });
  }

  // On initial load, update descriptions
  componentDidMount() {
    this.updateItem();
  }

  // On change of item id, update descriptions
  componentDidUpdate(prevProps, prevState) {
    // Update the item description, if it changed
    if (this.props.event.id !== prevProps.event.id) {
      this.updateItem();
    }
  }

  // Render the completed action
  render() {
    return (
      <>
        <div className={`selectedEvent ${this.state.isMenuVisible && "isEditing"}`} onClick={this.toggleMenu}>
          <div className="deleteEvent" onClick={(e) => {stopPropogation(e); this.props.changeEvent(0)}}>X</div>
          {this.state.description}
          <SendNode type="event" onPointerDown={(e) => {stopPropogation(e); this.props.grabFocus(this.props.event.id)}}/>
        </div>
      </>
    );
  }
}
