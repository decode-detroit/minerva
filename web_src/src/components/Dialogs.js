import React from 'react';

// A small dialog with a title and list of elements
export class SmallDialog extends React.PureComponent {
   // Class constructor
   constructor(props) {
    // Collect props
    super(props);

    // Bind the various functions
    this.handleMouseDown = this.handleMouseDown.bind(this);
  }
  
  // Function to prevent clicks from continuing
  handleMouseDown(e) {
    // Prevent the default event handler and propogation
    e = e || window.event;
    e.preventDefault();
    e.stopPropagation();
  }
  
  // Return the completed dialog
  render() {
    // Compose any items into a list
    const children = this.props.children.map((child) => child);
    
    // Return the dialog 
    return (
      <div className="smallDialog dialog" style={{ left: `${this.props.left}px`, top: `${this.props.top}px` }} onMouseDown={this.handleMouseDown}>
        <h3>{this.props.title}</h3>
        <div className="verticalList">{children}</div>
      </div>
    );
  }
}

// An event dialog with an event and event detail
export class EventDialog extends React.PureComponent {
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
    }

    // Bind the various functions
    this.updateEvent = this.updateEvent.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);
  }

  // Function to respond to clicking the area
  handleMouseDown(e) {
    // Prevent any other event handlers
    e = e || window.event;
    e.preventDefault();
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
  updateEvent() {
    // Fetch the details of the event
    fetch(`/getItem/${this.props.id}`)
    .then(response => {
      return response.json()
    })
    .then(json => {
      // If valid, save the result to the state
      if (json.item.isValid) {
        this.setState({
          itemPair: json.item.itemPair,
        });
      }
    });
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

  // Every time the id is updated, pull the event information
  /*componentDidUpdate(prevProps) {
    // If the id has not changed, exit FIXME this will need to be removed
    if (this.props.id !== prevProps.id) {
      this.updateEvent();
    }
  }*/
 
  // Return the completed dialog
  render() {
    // Return the dialog 
    return (
      <div className="eventDialog dialog" style={{ left: `${this.state.left}px`, top: `${this.state.top}px` }} onMouseDown={this.handleMouseDown}>
        <h3>{this.state.itemPair.description} ({this.state.itemPair.id})</h3>
      </div>
    );
  }
}

