use anyhow::Result;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use super::Peer;

impl Peer {
    /// Set the remote session description and generate an answer
    pub async fn set_remote_description(
        &self,
        offer: RTCSessionDescription,
    ) -> Result<RTCSessionDescription> {
        // Set the remote description
        self.connection.set_remote_description(offer).await?;

        // Create an answer
        let answer = self.connection.create_answer(None).await?;

        // Create channel that is blocked until ICE Gathering is complete
        let mut gather_complete = self.connection.gathering_complete_promise().await;

        // Sets the LocalDescription, and starts our UDP listeners
        self.connection.set_local_description(answer).await?;

        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate
        let _ = gather_complete.recv().await;

        // Output the answer in base64 so we can paste it in browser
        // ! FIXME: unhandled panic!
        Ok(self.connection.local_description().await.unwrap())
    }
}
