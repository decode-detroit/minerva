import React from 'react';
import { stopPropogation } from './Functions';

// A confirm button to confirm selection before proceeding
export class ConfirmButton extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      isSelected: false,
      isConfirmed: false,
      buttonHeight: null,
      buttonWidth: null,
    };

    // Bind the various functions
    this.handleClick = this.handleClick.bind(this);

    // Create a reference for the element
    this.button = React.createRef();

    // Create a holder for the timeout
    this.timeout = null;
  }
  
  // Function to respond to clicking the area
  handleClick(e) {
    stopPropogation(e);

    // If the button is already selected
    if (this.state.isSelected) {
      // Trigger the specified callback
      this.props.onClick();

      // Mark as triggered and remove selected
      this.setState({
        isSelected: false,
        isConfirmed: true,
      });

      // Clear any existing timeout
      if (this.timeout) {
        clearTimeout(this.timeout);
      }

      // Set the timeout to return to normal
      this.timeout = setTimeout(() => {
        // Reset the state and remove the timeout
        this.setState({
          isSelected: false,
          isConfirmed: false,
        })
        this.timeout = null;
      }, 1000);
    
    // Otherwise, select the button
    } else {
      // Mark as selected
      this.setState({
        isSelected: true,
        isConfirmed: false,
      });

      // Clear any existing timeout
      if (this.timeout) {
        clearTimeout(this.timeout);
      }

      // Set the timeout to return to normal
      this.timeout = setTimeout(() => {
        // Reset the state and remove the timeout
        this.setState({
          isSelected: false,
          isConfirmed: false,
        })
        this.timeout = null;
      }, 1000);
    }
  }

  // On initial load, lock the button size
  componentDidMount() {
    this.setState({
      buttonHeight: this.button.current.offsetHeight,
      buttonWidth: this.button.current.offsetWidth + 1,
    });
  }
 
  // Return the selected box
  render() {
    // Return the item box
    return (
      <button className={`${this.props.buttonClass}` + (this.state.isSelected ? " red" : "") + (this.state.isConfirmed ? " green" : "")} ref={this.button} style={{ height: `${this.state.buttonHeight}px`, width: `${this.state.buttonWidth}px` }} onClick={this.handleClick}>
        {!this.state.isSelected && `${this.props.buttonText}`}
        {this.state.isSelected && "Confirm?"}
      </button>
    );
  }
}

// An item button to  cue the selected item
export class ItemButton extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      description: "Loading ...", // placeholder for the real data
      type: "",
    };

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown(e) {
    stopPropogation(e);

    // Trigger the selected event FIXME should be moved to event subtype
    let cueEvent = {
      id: this.props.id,
      secs: 0,
      nanos: 0,
    };
    fetch(`/cueEvent`, {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
      body: JSON.stringify(cueEvent),
    }); // FIXME ignore errors
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        // Save the itemPair
        this.setState({
          description: json.data.item.description,
        });
      }

      // Check to see the item type
      response = await fetch(`getType/${this.props.id}`);
      const json2 = await response.json();

      // If valid, save the result to the state
      if (json2.generic.isValid) {
        this.setState({
          type: json2.generic.message,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the location and item information
  componentDidMount() {
    // Pull the item information
    this.updateItem();
  }
 
  // Return the selected box
  render() {
    // Return the item box
    return (
      <>
        {this.state.type !== "" &&
          <div id={`id-${this.props.id}`} className={`box ${this.state.type} row${this.props.row} ${this.props.isFocus ? 'focus' : ''}`} onMouseDown={this.handleMouseDown}>
            <div className="title">
              <div>{this.state.description}</div>
              <div className="disableSelect">({this.props.id})</div>
            </div>
          </div>
        }
      </>
    );
  }
}

