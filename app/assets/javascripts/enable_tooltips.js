$( document ).on('turbolinks:load', function() {
    $('[data-toggle="tooltip"]').tooltip(
        {
            'delay': { show: 800 },
            'placement': 'bottom'
        })
})