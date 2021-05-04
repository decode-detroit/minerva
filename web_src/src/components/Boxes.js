import React from 'react';
import { Action, ReceiveNode } from './Nodes';

// An event box with an event and event detail
export class EventBox extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      left: 0, // horizontal offset of the area
      top: 0, // vertical offest of the area
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
      itemPair: { // placeholder for the real data
        id: 0,
        description: "Loading ...",
      },
      eventActions: [], // placeholder for the read data
    }

    // Bind the various functions
    this.updateEvent = this.updateEvent.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);
  }

  // Function to respond to clicking the area
  handleMouseDown(e) {
    // Prevent propogation
    e = e || window.event;
    e.stopPropagation();
   
    // Connect the mouse event handlers to the document
    document.onmousemove = this.handleMouseMove;
    document.onmouseup = this.handleMouseClose;

    // Save the cursor position, hide the menu
    this.setState({
      cursorX: e.clientX,
      cursorY: e.clientY,
    });
  }

  // Function to respond to dragging the area
  handleMouseMove(e) {
    // Prevent the default event handler
    e = e || window.event;
    e.preventDefault();

    // Update the state
    this.setState((state) => {
      // Calculate change from old cursor position
      let changeX = state.cursorX - e.clientX;
      let changeY = state.cursorY - e.clientY;
  
      // Calculate the new location
      let left = state.left - changeX;
      let top = state.top - changeY;
  
      // Enforce bounds on the new location
      left = (left >= 0) ? left : 0;
      top = (top >= 0) ? top : 0;
  
      // Save the new location and current cursor position
      return {
        left: left,
        top: top,
        cursorX: e.clientX,
        cursorY: e.clientY,
      }
    });
  }
  
  // Function to respond to releasing the mouse
  handleMouseClose() {
    // Stop moving when mouse button is released
    document.onmousemove = null;
    document.onmouseup = null;
  }

  // Helper function to update the event information
  async updateEvent() {
    try {
      // Fetch the description of the event
      let response = await fetch(`/getItem/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.item.isValid) {
        this.setState({
          itemPair: json.item.itemPair,
        });
      }

      // Fetch the detail of the event
      response = await fetch(`getEvent/${this.props.id}`);
      const json2 = await response.json();

      // If valid, save the result to the state
      if (json2.event.isValid) {
        this.setState({
          eventActions: json2.event.event,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the location and event information
  componentDidMount() {
    // Set the starting location
    this.setState({
      left: this.props.left,
      top: this.props.top,
    })

    // Pull the new event information
    this.updateEvent();
  }
 
  // Return the completed box
  render() {
    // Compose any actions into a list
    const children = this.state.eventActions.map((action) => <Action key={action.toString()} action={action} createConnector={this.props.createConnector}></Action>);

    // Return the box
    return (
      <div className="box eventBox" style={{ left: `${this.state.left}px`, top: `${this.state.top}px` }} onMouseDown={this.handleMouseDown}>
        <div className="title">{this.state.itemPair.description} ({this.state.itemPair.id})</div>
        <ReceiveNode id={`receive-node-${this.state.itemPair.id}`} type="event"></ReceiveNode>
        <div className="subtitle">Actions:</div>
        <div className="verticalList">{children}</div>
      </div>
    );
  }
}

