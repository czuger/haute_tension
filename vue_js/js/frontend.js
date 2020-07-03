new Vue({
    el: "#current-page",
    data () {
        return {
            current_hash: null,
            texts_data: null
        }
    },
    methods: {
        getUrl: function ( hash ) {
            this.current_hash = hash;
            const h = LsManager.set_value( 'alamuth_current_hash', 'hash', hash );

            let h_history = LsManager.get_value( 'alamuth_hash_history', 'history' ) || [];
            h_history.push( hash );
            LsManager.set_value( 'alamuth_hash_history', 'history', h_history );
        },
        reset: function () {
            this.current_hash = '8ffd30fe683b4e639114068564522932464f4ba0170ad65f02c7897b85d55a3d';
        },
        back: function () {
            let h_history = LsManager.get_value( 'alamuth_hash_history', 'history' ) || [];
            h_history.pop();
            const previous_hash = h_history.pop();
            LsManager.set_value( 'alamuth_hash_history', 'history', h_history );
            this.current_hash = previous_hash;
        }
    },
    watch: {
        current_hash: function( newHash, oldHash ) {
            // Tricky, this is not available inside the axios function and has to be
            // assigned to self outside the call.
            var self = this;
            axios
                .get(`data/pretre_jean_forteresse_alamuth/${this.current_hash}.json`)
                .then( function( response ) {
                    // console.log( response.data.paragraphs );
                    self.texts_data = response.data.paragraphs;
                });
        }
    },
    mounted () {
        const h = LsManager.get_value( 'alamuth_current_hash', 'hash' );

        this.current_hash = h || '8ffd30fe683b4e639114068564522932464f4ba0170ad65f02c7897b85d55a3d';
    }
});