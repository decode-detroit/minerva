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

// An input for text with variable length based on the length of the input
export class TextInput extends React.PureComponent {
  // Return the text input
  render() {
    return (
      <>
        {!this.props.value && <input type="text" value="" size={20} onInput={this.props.handleInput} />}
        {this.props.value && <input type="text" value={this.props.value} size={this.props.value.length > 30 ? this.props.value.length - 10 : 20} onInput={this.props.handleInput} />}
      </>
    );
  }
}

// A toggle switch to change between two options
export class ToggleSwitch extends React.PureComponent {
  // Return the toggle switch
  render() {
    return (
      <span className="toggleSwitch" onClick={this.props.handleToggle}>
        <span className={`toggleOption ${!this.props.value ? "active" : ""}`}>{this.props.offOption}</span>
        <span className={`toggleOption ${this.props.value ? "active" : ""}`}>{this.props.onOption}</span>
      </span>
    );
  }
}
