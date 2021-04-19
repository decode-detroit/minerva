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
      <div className="smallDialog" style={{ left: `${this.props.left}px`, top: `${this.props.top}px` }} onMouseDown={this.handleMouseDown}>
        <h3>{this.props.title}</h3>
        <div className="verticalList">{children}</div>
      </div>
    );
  }
}
