import React from 'react';

// A simple link element
export function Link(props) {
  // Return the completed link
  return (
    <div>{props.text}</div>
  );
}

// An action list element
export class Action extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);

    // Bind the various functions
    /*this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);*/
  }

  // Function to respond to clicking the area
  /*handleMouseDown(e) {
    // Prevent any other event handlers
    e = e || window.event;
    e.preventDefault();
   
    // Connect the mouse event handlers to the document
    document.onmousemove = this.handleMouseMove;
    document.onmouseup = this.handleMouseClose;

    // Save the cursor position, hide the menu
    this.setState({
      cursorX: e.clientX,
      cursorY: e.clientY,
    });
  }*/

  // Render the draggable edit area
  render() {
    // Switch based on the props
    if (this.props.action.hasOwnProperty(`NewScene`)) {
      return (
        <div className="action">
          New Scene
          <SendNode type="scene"></SendNode>
        </div>
      );
    
    // Modify Status
    } else if (this.props.action.hasOwnProperty(`ModifyStatus`)) {
      return (
        <div className="action">
          Modify Status
          <SendNode type="status"></SendNode>
        </div>
      );
    
    // Cue Event
    } else if (this.props.action.hasOwnProperty(`CueEvent`)) {
      return (
        <div className="action">
          Cue Event
          <SendNode type="event"></SendNode>
        </div>
      );
    
    // Cancel Event
    } else if (this.props.action.hasOwnProperty(`CancelEvent`)) {
      return (
        <div className="action">
          Cancel Event
          <SendNode type="event"></SendNode>
        </div>
      );
    
    // Save Data
    } else if (this.props.action.hasOwnProperty(`SaveData`)) {
      return (
        <div className="action">
          Save Data
        </div>
      );
    
    // Send Data
    } else if (this.props.action.hasOwnProperty(`SendData`)) {
      return (
        <div className="action">
          Send Data
        </div>
      );

    // Select Event
    } else if (this.props.action.hasOwnProperty(`SelectEvent`)) {
      return (
        <div className="action">
          Select Event
          <SendNode type="status"></SendNode>
        </div>
      );
    }
    
    // Otherwise, return the default
    return (
        <div className="action">Invalid Action</div>
    );
  }
}

// A receive Node element
export function ReceiveNode(props) {
  // Return the completed link
  return (
    <div className={`node ${props.type}`} ></div>
  );
}

// A send Node element
export function SendNode(props) {
  // Return the completed link
  return (
    <div className={`node ${props.type}`} ></div>
  );
}