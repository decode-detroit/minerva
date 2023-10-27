import React from 'react';
import logoWide from './../logo_wide.png';
import { ConfirmButton } from './Buttons';
import { asyncForEach, switchPort } from './Functions';

// A header menu
export class HeaderMenu extends React.PureComponent {  
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      isFileVisible: false,
    }
  }

  // Render the header menu
  render() {
    return (
      <div className="header">
        <div className="headerLeft">
          <div className="title">Minerva</div>
          <div className={"menuButton" + (this.state.isFileVisible ? " selected" : "")} onClick={() => this.setState((prevState) => { return { isFileVisible: !prevState.isFileVisible }})}>File
            {this.state.isFileVisible &&
              <div class="headerExpansion">
                <ConfirmButton buttonClass="expansionMenuButton" onClick={() => {switchPort(64637);}} buttonText="Edit Mode" />
              </div>
            }
          </div>
          <SceneMenu value={this.props.currentScene.id} />
        </div>
        <div className="headerRight">
          <ConfirmButton buttonClass="menuButton" onClick={() => {this.props.closeMinerva();}} buttonText="Quit Minerva" />
          <img src={logoWide} className="logo" alt="logo" />
        </div>
      </div>
    );
  }
}

// A footer menu
export class FooterMenu extends React.PureComponent {  
  // Render the footer menu
  render() {
    return (
      <div className="footer">
        {this.props.notice && <>
          <div>{`Event: ${this.props.notice.message}`}</div>
          <div className="footnote">{`at ${this.props.notice.time}`}</div>
        </>}
      </div>
    );
  }
}

// A menu for the scene selection
export class SceneMenu extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      sceneList: [],
    }

    // Bind functions
    this.changeScene = this.changeScene.bind(this);
    this.handleChange = this.handleChange.bind(this);
  }

  // Function to change the displayed scene
  changeScene(id) {
    // Request the scene change
    let sceneChange = {
      sceneId: id,
    };
    fetch(`/sceneChange`, {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
      body: JSON.stringify(sceneChange),
    }); // FIXME ignore errors
  }

  // On render, pull the full scene list
  async componentDidMount() {
    try {
      // Fetch all scenes and process the response
      let response = await fetch(`/allScenes`);
      const json = await response.json();

      // If the response is valid
      if (json.isValid) {
        // Get the detail of each item
        let sceneList = [];
        await asyncForEach(json.data.items, async (item) => {
          // Fetch the description of the item
          response = await fetch(`/getItem/${item.id}`);
          const json2 = await response.json();

          // If valid, save the id and description
          if (json2.isValid) {
            sceneList.push({
              id: item.id,
              description: json2.data.item.description,
            });
          }
        });

        // Save the result to the state and prompt refresh
        this.setState({
          sceneList: sceneList,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Control the view of the component
  handleChange(e) {
    // Pass the change upstream
    this.changeScene(parseInt(e.target.value));
  }
  
  // Render the scene menu
  render() {
    // Compose the list of options
    let options = this.state.sceneList.map((scene) => <option key={scene.id.toString()} value={scene.id}>{scene.description}</option>);
    options.sort((first, second) => { return first.id - second.id } );

    // Return the complete menu
    return (
      <div className="sceneMenu">
        <div className="title">Current Scene:</div>
        <select className="select" value={this.props.value} onChange={this.handleChange}>
          {options}
        </select>
      </div>
    );
  }
}

