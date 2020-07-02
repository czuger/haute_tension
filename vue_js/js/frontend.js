new Vue({
    el: "#current-page",
    data () {
        return {
            current_hash: '8ffd30fe683b4e639114068564522932464f4ba0170ad65f02c7897b85d55a3d',
            texts_data: [ 'toto' ]
        }
    },
    mounted () {
        // Tricky, this is not available inside the axios function and has to be
        // assigned to self outside the call.
        var self = this;
        axios
            .get(`data/pretre_jean_forteresse_alamuth/${this.current_hash}.json`)
            .then( function( response ) {
                console.log( response.data.paragraphs );
                self.texts_data = response.data.paragraphs;
            });

        // $.get( `data/pretre_jean_forteresse_alamuth/${this.current_hash}.json`, function( data ) {
        //     console.log( data.paragraphs );
        //     this.texts = data.paragraphs
        // } );
    }
});