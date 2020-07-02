new Vue({
    el: "#production-hell",
    data: {
        currentTab: "Production",
        tabs: ["Production", "Usage", "Empire", "Faction"]
    },
    computed: {
        currentTabComponent: function() {
            return "tab-" + this.currentTab.toLowerCase();
        },
    },
    mount: {
        $.get( "ajax/test.html", function( data ) {
        $( ".result" ).html( data );
        alert( "Load was performed." );
    });
    }
});
